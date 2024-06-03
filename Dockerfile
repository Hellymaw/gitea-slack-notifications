FROM rust:1.78

WORKDIR /usr/src/intelli-gitea-notifications
COPY . .

RUN cargo install --path .

CMD [ "gitea-notif" ]