#!/usr/bin/env node
import "source-map-support/register";
import { App } from "aws-cdk-lib";
import { EmailImagesStack } from "../lib/email-images-stack";

const app = new App();
new EmailImagesStack(app, "HemnetEmailImagesStack", {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
});
