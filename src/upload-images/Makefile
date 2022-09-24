# Create this file with `AWS_PROFILE={aws-profile-name}` if you don't want to prefix all aws related commands
ENV_PATH=../../.env

ifneq ("$(wildcard $(ENV_PATH))","")
	include $(ENV_PATH)
	export
endif

# Run locally (not in Docker)
.PHONY: run-watch
run-watch:
	cargo watch -x 'run'

.PHONY: run
run:
	cargo run

# Invoke the lambda in AWS
.PHONY: invoke
invoke:
	# https://dev.to/nicholaschiasson/beginner-s-guide-to-running-rust-on-aws-lambda-277n
	aws lambda invoke --function-name hemnet-upload-images --invocation-type Event /dev/stdout | cat

.PHONY: clean-cdk
clean-cdk:
	rm -rf cdk/cdk.out

# Build and deploy the lambda to AWS
# TODO: add this to readme: AWS_PROFILE=junker npx cdk bootstrap
.PHONY: deploy
deploy:
	make clean-cdk && cd cdk && npx cdk deploy

.PHONY: deploy-and-invoke
deploy-and-invoke:
	make deploy && make invoke

