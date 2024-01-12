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
    ├── db_password.txt
    └── db_root_password.txt
```
Notes on each file:
- `.env_local` is optional and is for development purposes. Make sure the password matches what is in `db_root_password.txt`. It should contain the following:
    ```
    export ROCKET_PROFILE="debug"
    export ROCKET_DATABASES={sqlx={url="mysql://root:<db_root_password>@0.0.0.0:3306/blog"}}
    export ROCKET_ADMIN_HASH="..."
    export ROCKET_WRITING_DIR="./writing"
    export DATABASE_URL="mysql://root:bilboFan@0.0.0.0:3306/blog"
    ```
    The hashed password should be generated with the sha256 function, and this can be done with open ssl
    ```
    # with sha256sum
    echo -n foobar | sha256sum
    # with open ssl
    echo -n "foobar" | openssl dgst -sha256
    ```
    
- `.env_deploy` has the environment variables needed to run the server in the dockerised application. Its very similar to the one above, but the host name of the MySQL is different and the location of the TLS certificate and key are in a different place.
    ```
    export ROCKET_PROFILE="release"
    export ROCKET_DATABASES={sqlx={url="mysql://root:<db_root_password>@db:3306/blog"}}
    export ROCKET_ADMIN_HASH="..."
    export ROCKET_WRITING_DIR="/writing"
    export ROCKET_SECRET_KEY="..."
    ```
    The secret key can be generated with openssl
    ```
    openssl rand -base64 32
    ```
    The secret key will look something like `hPrYyЭRiMyµ5sBB1π+CMæ1køFsåqKvBiQJxBVHQk=`
- `db_root_password.txt` contains a password needed for MySQL.
- `db_password.txt` contains a password needed for MySQL.

  
### Running it with `docker`
This following two commands will start both the database and the server.
```console
$ docker compose build
$ docker compose up -d
``` 
I have seen an issue where the MySQL server hasn't fully started and the server will fail to start. If this happens, run the `up` command again.

You can verify this has worked by checking

```
curl http://0.0.0.0:8000
```
returns some HTML 
### Where are the uploaded documents stored

The location of where documents are stored is configured with the environment variable `ROCKET_WRITING_DIR`. While developing locally, it is convenient to store this data in a directory called "writing" in the top level directory of this project. This directory is mounted with a bind mount with the docker compose file, so the website will work if you run it with cargo or if you run it docker.

### Reverse proxy through nginx

nginx lets you set a reverse proxy and handle TLS in a rather straight forward way. Install nginx on the machine you want to serve the website from.

Generate a key and a certificate with

`key.pem` and `cert.pem` should be generated with something like openssl e.g.
```bash 
$ openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -sha256 -days 365
```
Create a configuration file in `/etc/nginx/conf.d/blog.conf` with the following contents
```
server {
    listen 443 ssl;

    server_name jhb.blog.test;
    
    ssl_certificate /home/joseph/nginx/certs/blog_cert.pem;
    ssl_certificate_key /home/joseph/nginx/keys/blog_key.pem;

    location / {
        proxy_pass http://localhost:8000/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_http_version 1.1;
    }
}
```
To view this while on your local network without registering any domain names or setting up your router in a special way, add this to the `/etc/hosts` file on the client machine.
```
# webdev tests
<IP of host machine> jhb.blog.test
```
You will now be able to access the site via https://jhb.blog.test - (ignore the warning that browsers give you!)

### Deploying the server on a development machine
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
Install the tailwind cli. Develop with:
```
./tailwindcss -i tailwind_src/input.css -o static/styles.css --watch;  
```

