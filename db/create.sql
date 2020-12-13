CREATE TABLE poll
(
    id character varying NOT NULL,
    name character varying NOT NULL,
    description character varying NOT NULL,
    owner_id character varying NOT NULL,
    expires timestamp with time zone NOT NULL,
    close timestamp with time zone,
    CONSTRAINT poll_pkey PRIMARY KEY (id)
);
CREATE INDEX expires_index ON poll USING btree
    (expires ASC NULLS LAST);
