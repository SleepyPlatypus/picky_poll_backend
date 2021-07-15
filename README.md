# Picky Poll backend
Rust backend for [Picky Poll](https://pickypoll.com).

# Developer Quickstart
```sh
# Initialize and run Postgres database
docker build db/ -t pickypoll-db
docker run -p 5432:5432 -e POSTGRES_PASSWORD=a pickypoll-db

# Run tests
PICKYPOLL_TEST_DB=postgresql://postgres:a@localhost:5432 cargo test

# Run service
PICKYPOLL_DB_URL=postgresql://postgres:a@localhost:5432 cargo watch -x run
# post an example poll
curl "localhost:8080/polls" -d @example-request.json -H "content-type: application/json" -i -H "x-vote-secret: test"
# retrieve it by GETting localhost:8080/polls/{poll_id}
```

# API
* POST /polls/
* GET /polls/{poll_id}
* PUT /polls/{poll_id}/ballots/{ballot_id}
