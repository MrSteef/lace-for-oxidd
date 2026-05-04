#!/usr/bin/env bash

.PHONY: miri
miri: miri.log
	bat $^
miri.log:
	MIRIFLAGS="-Zmiri-backtrace=full" cargo +nightly miri test lace_deque::tests::stress -- --nocapture 2>&1 | tee $@

perf.%: perf.data.%
	perf report -g -i $<
perf.data.stress:
	perf record -o $@ --call-graph=dwarf cargo test lace_deque::tests::stress
perf.data.%:
	perf record -o $@ --call-graph=dwarf cargo bench --bench $(patsubst .%,%,$(suffix $@)) -- -lace -w 4

.PHONY: clean
clean:
	-rm miri.log
	-rm perf.*
