.PHONY: build-ApiFunction

build-ApiFunction:
	cargo lambda build --release --arm64
	cp target/lambda/github-profile-api/bootstrap $(ARTIFACTS_DIR)/bootstrap
