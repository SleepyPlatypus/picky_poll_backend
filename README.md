# Quickstart
```sh
# Initialize and run Postgres database
docker build db/ -t pickypoll-db
docker run -p 5432:5432 -e POSTGRES_PASSWORD=a pickypoll-db
# Run tests
PICKYPOLL_TEST_DB=postgresql://postgres:a@localhost:5432 cargo test
