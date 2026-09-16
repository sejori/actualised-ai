import { describe, expect, it, vi } from 'vitest';
import { CompanyRunner, type RunnableCompany } from '../examples/company-runner';

const createClient = (settings: Record<string, unknown> = {}) => {
  let storedSettings = settings;
  const client: RunnableCompany = {
    start: vi.fn(async () => {}),
    getCompanySettings: vi.fn(async () => storedSettings),
    updateCompanySettings: vi.fn(async next => { storedSettings = next; }),
  };
  return { client, settings: () => storedSettings };
};

const createDelayGate = () => {
  const releases: Array<() => void> = [];
  let pending: (() => void) | undefined;
  return {
    delay: vi.fn(() => new Promise<void>(resolve => {
      releases.push(resolve);
      pending?.();
      pending = undefined;
    })),
    release: async () => {
      if (releases.length === 0) {
        await new Promise<void>(resolve => { pending = resolve; });
      }
      releases.shift()?.();
      await Promise.resolve();
      await Promise.resolve();
    },
  };
};

describe('CompanyRunner', () => {
  it('persists start and runs independently until paused', async () => {
    const { client, settings } = createClient({ provider: 'gemini' });
    const stateChanged = vi.fn();
    const gate = createDelayGate();
    const runner = new CompanyRunner(client, 100, stateChanged, gate.delay);

    await runner.start();
    await gate.release();
    await runner.pause();
    await gate.release();
    await runner.waitUntilIdle();

    expect(client.start).toHaveBeenCalledTimes(2);
    expect(settings()).toEqual({ provider: 'gemini', executionEnabled: false });
    expect(stateChanged).toHaveBeenCalledTimes(2);
    expect(runner.status()).toEqual({ enabled: false, running: false, lastError: null });
  });

  it('keeps start idempotent and never creates overlapping loops', async () => {
    const { client } = createClient();
    const gate = createDelayGate();
    const runner = new CompanyRunner(client, 100, undefined, gate.delay);

    await Promise.all([runner.start(), runner.start()]);
    await runner.pause();
    await gate.release();
    await runner.waitUntilIdle();

    expect(client.start).toHaveBeenCalledTimes(1);
  });

  it('restores persisted execution and records recoverable cycle errors', async () => {
    const { client } = createClient({ executionEnabled: true });
    const start = client.start as ReturnType<typeof vi.fn>;
    start.mockRejectedValueOnce(new Error('temporary failure')).mockResolvedValue(undefined);
    const gate = createDelayGate();
    const runner = new CompanyRunner(client, 100, undefined, gate.delay);

    await runner.restore();
    await Promise.resolve();
    expect(runner.status().lastError).toBe('temporary failure');
    await gate.release();
    expect(client.start).toHaveBeenCalledTimes(2);
    expect(runner.status().lastError).toBeNull();

    await runner.pause();
    await gate.release();
    await runner.waitUntilIdle();
  });

  it('stays idle when persisted execution is disabled', async () => {
    const { client } = createClient({ executionEnabled: false });
    const runner = new CompanyRunner(client);

    expect(await runner.restore()).toEqual({ enabled: false, running: false, lastError: null });
    expect(client.start).not.toHaveBeenCalled();
  });

  it('stops after a cycle disables execution through a root tool', async () => {
    const { client } = createClient({ executionEnabled: true });
    (client.start as ReturnType<typeof vi.fn>).mockImplementation(async () => {
      await client.updateCompanySettings({ executionEnabled: false });
    });
    const runner = new CompanyRunner(client, 100, undefined, vi.fn(async () => {}));

    await runner.restore();
    await runner.waitUntilIdle();

    expect(client.start).toHaveBeenCalledOnce();
    expect(runner.status().enabled).toBe(false);
  });
});