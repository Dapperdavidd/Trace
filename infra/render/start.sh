#!/bin/sh
set -eu

trace-worker &
worker_pid=$!

stop_worker() {
  kill "$worker_pid" 2>/dev/null || true
}

trap stop_worker EXIT INT TERM
exec trace-api
