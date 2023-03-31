# (Experimental) Testing with a Large Number of Repositories

This section explains how to run Kani on a large number of crates
downloaded from git forges. You may want to do this if you are going
to test Kani's ability to handle Rust features found in projects out
in the wild.

For the first half, we will explain how to use data from crates.io to
pick targets. Second half will explain how to use a script to run on a
list of selected repositories.

## Picking Repositories

In picking repositories, you may want to select by metrics like
popularity or by the presence of certain features. In this section, we
will explain how to select top ripostes by download count.

We will use the `db-dump` method of getting data from crates.io as it
is zero cost to their website and gives us SQL access. To start, have
the following programs set up on your computer.
- docker
- docker-compose.

1. Start PostgreSQL. Paste in the following yaml file as
`docker-compose.yaml`. `version: '3.3'` may need to change.
```yaml
version: '3.3'
services:
  db:
    image: postgres:latest
    restart: always
    environment:
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=postgres
    volumes:
      - crates-data:/var/lib/postgresql/data
    logging:
      driver: "json-file"
      options:
        max-size: "50m"
volumes:
  crates-data:
    driver: local
```
Then, run the following to start the setup.
```bash
docker-compose up -d
```

Once set up, run `docker ls` to figure out the container's name. We
will refer to the name as `$CONTAINER_NAME` from now on.

2. Download actual data from crates.io. First, run the following
   command to get a shell in the container: `docker exec -it --user
   postgres $CONTAINER_NAME bash`. Now, run the following to grab and
   install the data into the repository. Please note that this may
   take a while.

   ```bash
   wget https://static.crates.io/db-dump.tar.gz
   tar -xf db-dump.tar.gz
   psql postgres -f */schema.sql
   psql postgres -f */import.sql
   ```

3. Extract the data. In the same docker shell, run the following to
   extract the top 1k repositories. Other SQL queries may be used if
   you want another criteria

   ```sql
   \copy
   (SELECT name, repository, downloads  FROM crates
   WHERE repository LIKE 'http%' ORDER BY DOWNLOADS DESC LIMIT 1000)
   to 'top-1k.csv' csv header;
   ```

4. Clean the data. The above query will capture duplicates paths that
   are deeper than the repository. You can clean these out.
   - URL from CSV: `cat top-1k.csv | awk -F ',' '{ print $2 }' | grep -v 'http.*'`
   - Remove long paths: `sed 's/tree\/master.*$//g'`
   - Once processed, you can dedup with `sort | uniq --unique`

## Running the List of Repositories
In this step we will download the list of repositories using a script
[assess-scan-on-repos.sh](../../scripts/exps/assess-scan-on-repos.sh)

Make sure to have Kani ready to run. For that, see the [build instructions](cheat-sheets.md#build).

From the repository root, you can run the script with
`./scripts/exps/assess-scan-on-repos.sh $URL_LIST_FILE` where
`$URL_LIST_FILE` points to a line-delimited list of URLs you want to
run Kani on. Repositories that give warnings or errors can be grepping
for with "STDERR Warnings" and "Error exit in" respectively.
