# Create this file with `AWS_PROFILE={aws-profile-name}` if you don't want to prefix all aws related commands
AWS_ENV_PATH=../../aws.env

ifneq ("$(wildcard $(AWS_ENV_PATH))","")
	include $(AWS_ENV_PATH)
endif

RUN_CMD=run --package upload-images --bin upload-images

.PHONY: run-watch
run-watch:
	# https://crates.io/crates/cargo-watch
	cargo watch -x '$(RUN_CMD)'

.PHONY: run
run:
	cargo $(RUN_CMD)

.PHONY: local-invoke
local-invoke:
	# This was needed: `cdk synth --no-staging`
	# https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-cdk-getting-started.html
	# TODO: Check if template exists. If not, run `cdk synth` first
	sam local invoke upload-images --no-event --template ./cdk/cdk.out/UploadImagesLambdaStack.template.json

.PHONY: invoke
invoke:
	# https://dev.to/nicholaschiasson/beginner-s-guide-to-running-rust-on-aws-lambda-277n
	aws lambda invoke --function-name upload-images /dev/stdout | cat

.PHONY: clean-cdk
clean-cdk:
	rm -rf cdk/cdk.out

.PHONY: deploy
deploy:
	make clean-cdk && cd cdk && npx cdk deploy

.PHONY: deploy-and-invoke
deploy-and-invoke:
	make deploy && make invoke

