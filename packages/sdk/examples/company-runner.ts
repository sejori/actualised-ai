export type RunnableCompany = {
  start(): Promise<void>;
  getCompanySettings(): Promise<Record<string, unknown> | null>;
  updateCompanySettings(settings: Record<string, unknown>): Promise<void>;
};

export type CompanyRunnerStatus = {
  enabled: boolean;
  running: boolean;
  lastError: string | null;
};

export class CompanyRunner {
  private enabled = false;
  private running = false;
  private lastError: string | null = null;
  private loop: Promise<void> | null = null;

  constructor(
    private readonly client: RunnableCompany,
    private readonly intervalMs = 500,
    private readonly onStateChanged: () => void = () => {},
    private readonly delay: (milliseconds: number) => Promise<void> = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)),
  ) {}

  status(): CompanyRunnerStatus {
    return { enabled: this.enabled, running: this.running, lastError: this.lastError };
  }

  async restore(): Promise<CompanyRunnerStatus> {
    const settings = await this.client.getCompanySettings();
    this.enabled = settings?.executionEnabled === true;
    if (this.enabled) this.ensureLoop();
    return this.status();
  }

  async start(): Promise<CompanyRunnerStatus> {
    await this.persistEnabled(true);
    this.enabled = true;
    this.lastError = null;
    this.ensureLoop();
    return this.status();
  }

  async pause(): Promise<CompanyRunnerStatus> {
    this.enabled = false;
    await this.persistEnabled(false);
    return this.status();
  }

  async waitUntilIdle(): Promise<void> {
    await this.loop;
  }

  private async persistEnabled(enabled: boolean): Promise<void> {
    const settings = await this.client.getCompanySettings() ?? {};
    await this.client.updateCompanySettings({ ...settings, executionEnabled: enabled });
  }

  private ensureLoop(): void {
    if (this.loop) return;
    this.loop = this.runLoop().finally(() => {
      this.loop = null;
      if (this.enabled) this.ensureLoop();
    });
  }

  private async runLoop(): Promise<void> {
    while (this.enabled) {
      this.running = true;
      try {
        await this.client.start();
        this.lastError = null;
        const settings = await this.client.getCompanySettings();
        this.enabled = settings?.executionEnabled === true;
        this.onStateChanged();
      } catch (error) {
        this.lastError = error instanceof Error ? error.message : String(error);
        console.error('Company runner cycle failed:', this.lastError);
      } finally {
        this.running = false;
      }

      if (this.enabled) await this.delay(this.intervalMs);
    }
  }
}