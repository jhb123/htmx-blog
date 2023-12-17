# Intro
This is an experiement with `Docker`, `Rust`, the `Rocket` framework and `htmx`.

## Running the application

### Setting up secrets
This application is made of two parts: the server and the MySQL database. Before running the application, the secrets directory needs setting up. This must include the following files.
```
.
└── secrets/
    ├── .env_local
    ├── .env_deploy
    ├── cert.pem
    ├── db_password.txt
    ├── db_root_password.txt
    └── key.pem
```
Notes on each file:
- `.env_local` is optional and is for development purposes. Make sure the password matches what is in `db_root_password.txt`. It should contain the following:
    ```
    export ROCKET_PROFILE="debug"
    export ROCKET_DATABASES={sqlx={url="mysql://root:<db_root_password>@0.0.0.0:3306/blog"}}
    export ROCKET_TLS={certs="secrets/cert.pem",key="secrets/key.pem"}
    export ROCKET_ADMIN_HASH="..."
    export ROCKET_WRITING_DIR="./writing"
    ```
    The hashed password should be generated with the sha256 function, and this can be done on various websites.
- `.env_deploy` has the environment variables needed to run the server in the dockerised application. Its very similar to the one above, but the host name of the MySQL is different and the location of the TLS certificate and key are in a different place.
    ```
    export ROCKET_PROFILE="debug"
    export ROCKET_DATABASES={sqlx={url="mysql://root:<db_root_password>@db:3306/blog"}}
    export ROCKET_TLS={certs="/run/secrets/cert",key="/run/secrets/key"}
    export ROCKET_ADMIN_HASH="..."
    export ROCKET_WRITING_DIR="/writing"
    ```
- `db_root_password.txt` contains a password needed for MySQL.
- `db_password.txt` contains a password needed for MySQL.
- `key.pem` and `cert.pem` should be generated with something like openssl.
  
### Running it with `docker`
This following two commands will start both the database and the server.
```console
$ docker compose build
$ docker compose up -d
``` 
There may be an issue where the MySQL server hasn't fully started and the server will fail to start. If this happens, run the `up` command again.

### Where are the uploaded documents stored

The location of where documents are stored is configured with the environment variable `ROCKET_WRITING_DIR`. While developing locally, it is convenient to store this data in a directory called "writing" in the top level directory of this project. This directory is mounted with a bind mount with the docker compose file, so the website will work if you run it with cargo or if you run it docker.

### Deploying on the server development machine
This requires you to have cargo set up. First, you should start the MySQL server with docker compose
```console
$ docker compose up -d db
```
Next, set up the environment variables.
```console
$ source secrets/.env_local
```
Finally, build the server with e.g.
```
cargo run --bin server
```
## Development
### Database
Add a migration with `sqlx migrate add --source db/migrations <name of migrations>`

Never delete or modify the files in `db/migrations`. If you need a new migration, you must add them correctly otherwise you will mess up the MySQL database.
### tailwind
Install the tailwind cli. 
```
./tailwindcss -i input.css -o output.css --watch
```

