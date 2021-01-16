import sys, json;

j = json.loads(sys.stdin.read())

time_frag = {"time_and_filesize": str(int(j["filesize"]) * int(j["accessed"]))}

print(json.dumps(time_frag))
