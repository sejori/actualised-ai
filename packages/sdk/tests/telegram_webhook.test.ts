import { describe, expect, it, vi } from 'vitest';
import { app, processTelegramMessage } from '../examples/server';
import { ActualisedClient } from '../src/index';

describe('Telegram inference flow', () => {
  it('sends a plain-text model response when no structured Telegram call was emitted', async () => {
    const client = {
      queueMessage: vi.fn(async () => {}),
      start: vi.fn(async () => {}),
      getAgentContext: vi.fn(async () => ({ history: [{ role: 'agent', content: 'Hello from the root agent' }] })),
    };
    const sendMessage = vi.fn(async () => {});

    await processTelegramMessage(client, 'root', 'Hello', 'token', 123, sendMessage);

    expect(client.queueMessage).toHaveBeenCalledWith('root', expect.stringContaining('Hello'));
    expect(client.start).toHaveBeenCalledOnce();
    expect(sendMessage).toHaveBeenCalledWith('token', 123, 'Hello from the root agent');
  });

  it('does not duplicate a structured telegram_notify call', async () => {
    const client = {
      queueMessage: vi.fn(async () => {}),
      start: vi.fn(async () => {}),
      getAgentContext: vi.fn(async () => ({ history: [{ role: 'agent', content: 'Called tools: telegram_notify' }] })),
    };
    const sendMessage = vi.fn(async () => {});

    await processTelegramMessage(client, 'root', 'Hello', 'token', 123, sendMessage);

    expect(sendMessage).not.toHaveBeenCalled();
  });

  it('awaits processing failures and sends an error response before returning', async () => {
    const client = {
      getCompanySettings: vi.fn(async () => ({ telegramBotToken: 'dummy_token', telegramChatId: '123' })),
      updateCompanySettings: vi.fn(async () => {}),
      getAgents: vi.fn(async () => [{ id: 'root', parent_id: null }]),
      queueMessage: vi.fn(async () => { throw new Error('inference unavailable'); }),
      start: vi.fn(async () => {}),
      getAgentContext: vi.fn(async () => ({ history: [] })),
    };
    vi.spyOn(ActualisedClient, 'createSystemClient').mockResolvedValue(client as unknown as ActualisedClient);
    const fetchSpy = vi.spyOn(global, 'fetch').mockResolvedValue(new Response(JSON.stringify({ ok: true }), { status: 200 }));

    const response = await app.request('/api/webhooks/telegram/test_awaited_error', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message: { chat: { id: 123 }, text: 'Hello bot' } }),
    });

    expect(response.status).toBe(200);
    const errorCall = fetchSpy.mock.calls.find(call => String(call[0]).endsWith('/sendMessage'));
    expect(errorCall).toBeDefined();
    expect(JSON.parse(errorCall![1]!.body as string).text).toContain('inference unavailable');

    fetchSpy.mockRestore();
    vi.restoreAllMocks();
  });
});
