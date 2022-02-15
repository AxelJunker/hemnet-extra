import * as lambda from "aws-cdk-lib/aws-lambda";
import * as targets from "aws-cdk-lib/aws-events-targets";
import * as events from "aws-cdk-lib/aws-events";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import { Stack, StackProps, Duration } from "aws-cdk-lib";
import { Construct } from "constructs";
import * as path from "path";

export class UploadImagesLambdaStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const uploadImagesLambda = new lambda.DockerImageFunction(
      this,
      "UploadImagesHandler",
      {
        functionName: "hemnet-upload-images",
        code: lambda.DockerImageCode.fromImageAsset(
          path.join(__dirname, "../..")
        ),
        environment: {
          RUST_BACKTRACE: "1",
        },
      }
    );

    const eventRule = new events.Rule(this, "scheduleRule", {
      schedule: events.Schedule.rate(Duration.minutes(1)),
    });

    eventRule.addTarget(new targets.LambdaFunction(uploadImagesLambda));

    new dynamodb.Table(this, "HemnetImages", {
      partitionKey: { name: "id", type: dynamodb.AttributeType.NUMBER },
    });
  }
}
