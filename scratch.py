import urllib.request
import json
import ssl
import time

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

base_url = "https://actualised-orchestrator-yy32pk5s3q-nw.a.run.app"

# 1. Signup
print("Signing up...")
req1 = urllib.request.Request(f"{base_url}/api/auth/signup", data=json.dumps({"email": "admin@actualised.ai", "password": "password"}).encode("utf-8"), headers={"Content-Type": "application/json"})
try:
    with urllib.request.urlopen(req1, context=ctx) as res:
        data = json.loads(res.read().decode("utf-8"))
        token = data["token"]
        print("Got token:", token)
except Exception as e:
    print("Signup failed (maybe already exists). Trying signin...")
    req_signin = urllib.request.Request(f"{base_url}/api/auth/signin", data=json.dumps({"email": "admin@actualised.ai", "password": "password"}).encode("utf-8"), headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req_signin, context=ctx) as res:
            data = json.loads(res.read().decode("utf-8"))
            token = data["token"]
            print("Got token from signin:", token)
    except Exception as e:
        print("Signin failed too:", e)
        if hasattr(e, 'read'): print(e.read())
        exit(1)

# Note: Since the Cloud Run server is broken for CompanyState::init with normal tokens right now,
# wait! If the Cloud Run server is currently broken, it will STILL throw the IAM error because I haven't deployed the fix!
# I can't hit the Cloud Run server to create the company until I deploy!
