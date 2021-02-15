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
    id serial,
    CONSTRAINT candidate_pkey PRIMARY KEY (id),

    name character varying NOT NULL,
    description character varying,

    poll_id character varying NOT NULL,

    CONSTRAINT candidate_poll_fkey FOREIGN KEY (poll_id)
        REFERENCES poll (id)
        ON UPDATE RESTRICT
        ON DELETE CASCADE,

    UNIQUE (name, poll_id)
);
CREATE INDEX fki_candidate_poll_fkey
    ON candidate(poll_id);

--BALLOT--
CREATE TABLE ballot
(
    id character varying NOT NULL,

    name character varying NOT NULL,
    timestamp timestamp with time zone NOT NULL,
    owner_id character varying NOT NULL,

    poll_id character varying NOT NULL,

    CONSTRAINT ballot_pkey PRIMARY KEY (id, poll_id),

    CONSTRAINT ballot_poll_fkey FOREIGN KEY (poll_id)
        REFERENCES poll (id)
        ON UPDATE RESTRICT
        ON DELETE CASCADE
);

CREATE INDEX fki_ballot_poll_fkey
    ON ballot(poll_id);

--BALLOT_RANKING--
CREATE TABLE ranking
(
    ballot_id character varying NOT NULL,
    poll_id character varying NOT NULL,
    candidate_id integer NOT NULL,
    ranking smallint NOT NULL,

    CONSTRAINT ranking_ballot_poll_fkey FOREIGN KEY (ballot_id, poll_id)
        REFERENCES ballot (id, poll_id)
        ON UPDATE RESTRICT
        ON DELETE CASCADE,

    CONSTRAINT ranking_candidate_fkey FOREIGN KEY (candidate_id)
        REFERENCES candidate (id)
        ON UPDATE RESTRICT
        ON DELETE CASCADE,

    CONSTRAINT ranking_pkey PRIMARY KEY (ballot_id, poll_id, candidate_id),

    UNIQUE (ballot_id, ranking)
);
CREATE INDEX fki_ranking_ballot_poll_fkey
    ON ranking(ballot_id, poll_id);

CREATE INDEX fki_ranking_poll_fkey
    ON ranking(poll_id);