CREATE TABLE processed_signatures (
    signature   BYTEA       NOT NULL,
    block_time  BIGINT,
    PRIMARY KEY (signature)
);

CREATE TABLE diaries (
    account         VARCHAR(44)             NOT NULL,
    user_address    VARCHAR(44)             NOT NULL,
    name            VARCHAR(44)             NOT NULL,
    signature       BYTEA                   NOT NULL,
    raw_transaction BYTEA                   NOT NULL,
    PRIMARY KEY (account)
);

CREATE TABLE records (
    account         VARCHAR(44)             NOT NULL,
    diary           VARCHAR(44)             NOT NULL,
    text            VARCHAR                 NOT NULL,
    signature       BYTEA                   NOT NULL,
    raw_transaction BYTEA                   NOT NULL,
    PRIMARY KEY (account)
);
