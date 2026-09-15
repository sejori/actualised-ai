import urllib.request, json, ssl
ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

TELEGRAM_BOT_TOKEN = "8212574150:AAGeoV3ZMfvF-hTDR_cllh3B9PaF0-aUa84"
SURREALDB = "https://actualised-db-06g8gatlddrq53037u2ngf871c.aws-euw1.surreal.cloud"
CLOUD_RUN = "https://actualised-orchestrator-yy32pk5s3q-nw.a.run.app"

# Sign in to get fresh token
req = urllib.request.Request(f"{SURREALDB}/signin",
    data=json.dumps({"NS":"actualised","DB":"core","AC":"user","email":"seb@actualised.ai","pass":"password123"}).encode(),
    headers={"Content-Type":"application/json","Accept":"application/json"})
with urllib.request.urlopen(req, context=ctx) as r:
    fresh_token = json.loads(r.read())["token"]
    print("Got fresh token:", fresh_token[:40], "...")

# Verify the token works
req2 = urllib.request.Request(f"{SURREALDB}/sql", data=b"RETURN $auth.id;", headers={
    "Accept": "application/json",
    "Authorization": f"Bearer {fresh_token}",
    "NS": "actualised", "DB": "core"
})
with urllib.request.urlopen(req2, context=ctx) as r:
    print("Token valid, auth id:", json.loads(r.read()))

# Update the Telegram webhook
webhook_url = f"{CLOUD_RUN}/api/webhooks/telegram?token={fresh_token}"
req3 = urllib.request.Request(
    f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/setWebhook",
    data=json.dumps({"url": webhook_url}).encode(),
    headers={"Content-Type":"application/json"})
with urllib.request.urlopen(req3, context=ctx) as r:
    print("Webhook updated:", json.loads(r.read()))
