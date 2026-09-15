import urllib.request
import json
import ssl
ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJpYXQiOjE3ODk0MjE5NzEsIm5iZiI6MTc4OTQyMTk3MSwiZXhwIjoxNzkyMDEzOTcxLCJpc3MiOiJTdXJyZWFsREIiLCJqdGkiOiI0MTM1NTUxOC0zMTM1LTQ4ZjAtYTMyMi00MWI1YWQ4YmMwNjkiLCJOUyI6ImFjdHVhbGlzZWQiLCJEQiI6ImNvcmUiLCJBQyI6InVzZXIiLCJJRCI6InVzZXI6dXdiOHdwNDlpdXhvc2JkbHZ5OGIifQ.YWBTcQLNo477bC6jxJwt1I5NOJP6D_5pHSzezjeWVbB1vx49HOp0mp2jl_6rdOaYzZY7NaaNvdeFMUAOmKRx1g"
url = f"https://actualised-orchestrator-yy32pk5s3q-nw.a.run.app/api/company?token={token}"
payload = {"name": "Actualised HQ"}
req = urllib.request.Request(url, data=json.dumps(payload).encode("utf-8"), headers={"Content-Type": "application/json"})
with urllib.request.urlopen(req, context=ctx) as response:
    print(response.read().decode("utf-8"))
