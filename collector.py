from flask import Flask, request
import json

app = Flask(__name__)

try:
    with open("./db.json") as f:
        logs = f.read()
        logs = json.loads(logs)
except Exception as e:
    print(e)
    logs = []

@app.route("/collect", methods=["POST"])
def collect():
    print("Called collect")
    data = request.json
    logs.append(data)
    return ('', 204)

@app.route("/db")
def showDB():
    return (json.dumps(logs), 200)

@app.route("/savedb")
def saveDB():
    with open("./db.json", "w") as f:
        f.write(json.dumps(logs))
    return ('', 204)

app.run("0.0.0.0")
