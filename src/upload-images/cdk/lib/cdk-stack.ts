import * as cdk from "@aws-cdk/core";
import * as lambda from "@aws-cdk/aws-lambda";
import * as targets from "@aws-cdk/aws-events-targets";
import * as events from "@aws-cdk/aws-events";
import * as path from "path";

export class UploadImagesLambdaStack extends cdk.Stack {
  constructor(scope: cdk.Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const uploadImagesLambda = new lambda.DockerImageFunction(
      this,
      "UploadImagesHandler",
      {
        functionName: "upload-images",
        code: lambda.DockerImageCode.fromImageAsset(
          path.join(__dirname, "../..")
        ),
        // environment: {
        //   DYNAMODB_TABLE: this.table.tableName,
        // },
      }
    );

    const eventRule = new events.Rule(this, "scheduleRule", {
      schedule: events.Schedule.rate(cdk.Duration.minutes(1)),
    });

    eventRule.addTarget(new targets.LambdaFunction(uploadImagesLambda));
  }
}
