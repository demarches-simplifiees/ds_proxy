import urllib.request
from flask import Flask, request, send_from_directory
app = Flask(__name__)

@app.route("/<path:filename>")
def serve_uploaded_file(filename):
    return send_from_directory("uploads", filename)

@app.route('/<path:filename>', methods=['PUT'])
def upload_file(filename):
    file = open("uploads/{}".format(filename), 'wb')
    file.write(request.data)
    file.close()

    return 'OK!', 200, {'ContentType':'text/plain'}

if __name__ == "__main__":
    app.run()