CREATE TABLE threads (
    url varchar,
    ts varchar
);

CREATE TABLE user_lookup (
    gitea_tag varchar PRIMARY KEY,
    slack_uid varchar UNIQUE NOT NULL
);
