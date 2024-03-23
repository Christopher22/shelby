# Shelby
Management you association properly just insider your browser.

## Deployment

### During development
The application is developed inside a development container. Please open the repository with Visual Studio Code and open it as development container. For starting the server, type `cargo run` into the corresponding terminal.

### In productivity
Install Docker or another software to run containers. Than, open the terminal, navigate to the root folder of this repository, and run `docker build -t shelby:0.1 .`. After the build is complete, use `docker run -p 8080:8080 --mount type=bind,source="FOLDER WHERE database.sqlite WILL BE STORED",target=/data shelby:0.1` to run the container and access it under "http://localhost:8000".
