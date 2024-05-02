FROM rust:1.67

WORKDIR /usr/src/intelli-gitea-notifications
COPY . .

RUN cargo install --path .

CMD [ "gitea-notif" ]