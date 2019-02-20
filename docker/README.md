docker
======

Bulid
-----

```console
$ docker build -t plumtuna .

$ docker run --rm -it --name plumtuna-contact plumtuna sh -c 'plumtuna --rpc-addr $(hostname -i):7364'

$ HOST=$(docker inspect --format="{{ .NetworkSettings.IPAddress }}" plumtuna-contact)
$ docker run --rm -it plumtuna python3 /plumtuna.py/examples/bench.py --contact=$HOST --port=7364
```

Plumtuna Storage Benchmark
--------------------------

```console
$ docker-compose -f bench-plumtuna.yml up
$ docker exec docker_optimize0_1  sar -n DEV 5
$ docker stats
```

PostgreSQL Storage Benchmark
----------------------------

```console
$ docker-compose -f bench-postgres.yml up
```
