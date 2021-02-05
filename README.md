# Picky Poll backend
Porting the backend of [Picky Poll](https://pickypoll.com) to Rust.

This port implements Creating+Reading polls. It is useless, until it allows creating ballots.
# Developer Quickstart
```sh
# Initialize and run Postgres database
docker build db/ -t pickypoll-db
docker run -p 5432:5432 -e POSTGRES_PASSWORD=a pickypoll-db

# Run tests
PICKYPOLL_TEST_DB=postgresql://postgres:a@localhost:5432 cargo test

# Run paths & post example request
PICKYPOLL_DB_URL=postgresql://postgres:a@localhost:5432 cargo watch -x run
curl "localhost:8080/polls" -d @example-request.json -H "content-type: application/json" -i -H "secret-key: test"
# retrieve it by GETting localhost:8080/polls/{poll_id}
```

# API
## Partially Implemented
* POST /polls/
* GET /polls/{poll_id}
## Not Implemented
* PUT /polls/{poll_id}/ballots/{ballot_id}
