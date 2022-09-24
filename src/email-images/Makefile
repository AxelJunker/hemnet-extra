# Create this file with `AWS_PROFILE={aws-profile-name}` if you don't want to prefix all aws related commands
ENV_PATH=../../.env

ifneq ("$(wildcard $(ENV_PATH))","")
	include $(ENV_PATH)
	export
endif

# Run locally
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
	aws lambda invoke --function-name hemnet-email-images --invocation-type Event --cli-binary-format raw-in-base64-out --payload file://example-events/example-1.json /dev/stdout | cat

.PHONY: clean-cdk
clean-cdk:
	rm -rf cdk/cdk.out

# Build and deploy the lambda to AWS
# TODO: add this to readme: AWS_PROFILE=junker npx cdk bootstrap
# TODO: add this to readme: You will get an email from AWS saying you will need to verify your email address
.PHONY: deploy
deploy:
	make clean-cdk && cd cdk && npx cdk deploy

.PHONY: deploy-and-invoke
deploy-and-invoke:
	make deploy && make invoke

