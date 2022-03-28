#!/usr/bin/env node
import "source-map-support/register";
import { App } from "aws-cdk-lib";
import { UploadImagesStack } from "../lib/upload-images-stack";

const app = new App();
new UploadImagesStack(app, "HemnetUploadImagesStack");
