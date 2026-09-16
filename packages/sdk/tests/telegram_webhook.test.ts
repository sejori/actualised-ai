import { describe, expect, it, vi } from 'vitest';
import { app } from '../examples/server';
import { ActualisedClient } from '../src/index';

describe('Telegram Webhook Error Handling', () => {
  it('should catch orchestrator errors and send them to the user via Telegram', async () => {
    // Mock global fetch to spy on the outgoing telegram api requests
    const fetchSpy = vi.spyOn(global, 'fetch').mockImplementation(async (url: any, options: any) => {
      return new Response(JSON.stringify({ ok: true }));
    });

    // 1. Create a client and seed a company
    const client = await ActualisedClient.createSystemClient('mem://', 'company:test_telegram');
    await client.foundCompany('Telegram Test Company');
    // Also we need to add a root agent for the webhook to find
    await client.addAgent({
      id: 'agent_root',
      name: 'Root Agent',
      role: 'Manager',
      system_prompt: 'You manage the system',
      tools: ['telegram_notify']
    });

    // Mock createSystemClient to reuse our memory DB instance
    vi.spyOn(ActualisedClient, 'createSystemClient').mockResolvedValue(client);
    
    // Set up dummy telegram credentials in the DB
    await client.updateCompanySettings({
      telegramBotToken: 'dummy_token',
      telegramChatId: '123456789'
    });
    
    // Set up bad API config so inference fails
    await client.configureInference({
      provider: 'gemini',
      api_key: 'invalid_key',
      model: 'gemini-1.5-pro'
    });

    // 2. Simulate incoming Telegram webhook via Hono app
    const res = await app.request('/api/webhooks/telegram/test_telegram', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        message: {
          chat: { id: 123456789 },
          text: 'Hello bot'
        }
      })
    });

    expect(res.status).toBe(200);

    // Wait for the async setTimeout in server.ts to complete and rust to hit Gemini API
    await new Promise((resolve) => setTimeout(resolve, 3000));

    // 3. Verify that fetch was called to send the error message!
    expect(fetchSpy).toHaveBeenCalledWith(
      'https://api.telegram.org/botdummy_token/sendChatAction',
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('typing')
      })
    );
    
    const sendMsgCalls = fetchSpy.mock.calls.filter(call => call[0] === 'https://api.telegram.org/botdummy_token/sendMessage');
    expect(sendMsgCalls.length).toBeGreaterThan(0);
    
    const errorPayload = JSON.parse(sendMsgCalls[0][1].body as string);
    expect(errorPayload.text).toContain('System Error');

    fetchSpy.mockRestore();
  });
});
