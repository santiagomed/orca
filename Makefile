# Makefile

.PHONY: tests start_qdrant stop_qdrant

tests: start_qdrant run_tests stop_qdrant

start_qdrant:
	@echo "Starting Qdrant via Docker..."
	# Pull Qdrant image from DockerHub
	docker pull qdrant/qdrant
	# Run Qdrant in detached mode (in the background)
	docker run --name qdrant_test_instance -p 6333:6333 -p 6334:6334 \
    -e QDRANT__SERVICE__GRPC_PORT="6334" \
    qdrant/qdrant

run_tests:
	@echo "Running tests..."
	cargo test

stop_qdrant:
	@echo "Stopping Qdrant Docker instance..."
	docker stop qdrant_test_instance
	docker rm qdrant_test_instance
