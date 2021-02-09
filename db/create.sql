--POLL--
CREATE TABLE poll
(
    id character varying NOT NULL,
    CONSTRAINT poll_pkey PRIMARY KEY (id),
    name character varying NOT NULL,
    description character varying NOT NULL,
    owner_id character varying NOT NULL,
    expires timestamp with time zone NOT NULL,
    close timestamp with time zone
);
CREATE INDEX expires_index ON poll USING btree
    (expires ASC NULLS LAST);

--CANDIDATE--
CREATE TABLE candidate
(
    id SERIAL,
    CONSTRAINT candidate_pkey PRIMARY KEY (id),

    name character varying NOT NULL,
    description character varying,

    poll_id character varying NOT NULL
);

ALTER TABLE candidate
    ADD CONSTRAINT candidate_poll_fkey FOREIGN KEY (poll_id)
    REFERENCES poll (id)
    ON UPDATE RESTRICT
    ON DELETE CASCADE;

CREATE INDEX fki_candidate_poll_fkey
    ON candidate(poll_id);

ALTER TABLE candidate
    ADD UNIQUE (name, poll_id);

--BALLOT--
CREATE TABLE ballot
(
    id character varying NOT NULL,

    name character varying NOT NULL,
    timestamp timestamp with time zone NOT NULL,
    owner_id character varying NOT NULL,

    poll_id character varying NOT NULL,

    CONSTRAINT ballot_pkey PRIMARY KEY (id, poll_id)
);

ALTER TABLE ballot
    ADD CONSTRAINT ballot_poll_fkey FOREIGN KEY (poll_id)
    REFERENCES poll (id)
    ON UPDATE RESTRICT
    ON DELETE CASCADE;

CREATE INDEX fki_ballot_poll_fkey
    ON ballot(poll_id);