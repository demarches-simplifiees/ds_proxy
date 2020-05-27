import os
import urllib.request
from flask import Flask, flash, request, redirect, render_template, send_from_directory
app = Flask(__name__)

@app.route("/<path:filename>")
def serve_uploaded_file(filename):
    return send_from_directory("uploads", filename)

@app.route('/', methods=['PUT'])
def upload_file():
    # print(request.url)
    pass
    # if request.method == 'PUT':
    #     # check if the post request has the file part
    #     if 'file' not in request.files:
    #         flash('No file part')
    #         return redirect(request.url)
    #     file = request.files['file']
    #     if file.filename == '':
    #         flash('No file selected for uploading')
    #         return redirect(request.url)
    #     if file and allowed_file(file.filename):
    #         filename = secure_filename(file.filename)
    #         file.save(os.path.join(app.config['UPLOAD_FOLDER'], filename))
    #         flash('File successfully uploaded')
    #         return redirect('/')
    #     else:
    #         flash('Allowed file types are txt, pdf, png, jpg, jpeg, gif')
    #         return redirect(request.url)

if __name__ == "__main__":
    app.run()