# Create this file with `AWS_PROFILE={aws-profile-name}` if you don't want to prefix all aws related commands
AWS_ENV_PATH=../../aws.env

ifneq ("$(wildcard $(AWS_ENV_PATH))","")
	include $(AWS_ENV_PATH)
endif

.PHONY: run-watch
run-watch:
	# https://crates.io/crates/cargo-watch
	cargo watch -x 'run'

# Run locally (not in Docker)
.PHONY: run
run:
	cargo run

# Run this after changing anything in the cdk (and you want to run local-invoke)
.PHONY: synth
synth:
	cdk synth --no-staging	

# Run this after changing function code (and you want to run local-invoke)
.PHONY: build
build:
	sam build --template ./cdk/cdk.out/UploadImagesLambdaStack.template.json

# Invoke the lambda locally (in Docker)
.PHONY: local-invoke
local-invoke:
	# https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-cdk-getting-started.html
	# TODO: Check if template exists. If not, run `cdk synth` first
	sam local invoke hemnet-upload-images --no-event --template ./cdk/cdk.out/UploadImagesLambdaStack.template.json

# Invoke the lambda in AWS
.PHONY: invoke
invoke:
	# https://dev.to/nicholaschiasson/beginner-s-guide-to-running-rust-on-aws-lambda-277n
	aws lambda invoke --function-name hemnet-upload-images --invocation-type Event /dev/stdout | cat

.PHONY: clean-cdk
clean-cdk:
	rm -rf cdk/cdk.out

# Build and deploy the lambda to AWS
.PHONY: deploy
deploy:
	make clean-cdk && cd cdk && npx cdk deploy

.PHONY: deploy-and-invoke
deploy-and-invoke:
	make deploy && make invoke

