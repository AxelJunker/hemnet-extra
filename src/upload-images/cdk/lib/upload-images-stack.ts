import * as lambda from "aws-cdk-lib/aws-lambda";
import * as targets from "aws-cdk-lib/aws-events-targets";
import * as events from "aws-cdk-lib/aws-events";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as s3 from "aws-cdk-lib/aws-s3";
import { Stack, StackProps, Duration } from "aws-cdk-lib";
import { Construct } from "constructs";
import * as path from "path";

export class UploadImagesStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    // Lambda
    const uploadImagesLambda = new lambda.DockerImageFunction(
      this,
      "HemnetUploadImagesHandler",
      {
        functionName: "hemnet-upload-images",
        code: lambda.DockerImageCode.fromImageAsset(
          path.join(__dirname, "../..")
        ),
        environment: {
          RUST_BACKTRACE: "1",
        },
        memorySize: 1024,
        timeout: Duration.minutes(5),
      }
    );

    // EventBridge schedule rule
    const eventRule = new events.Rule(this, "ScheduleRule", {
      schedule: events.Schedule.rate(Duration.days(1)),
    });

    eventRule.addTarget(new targets.LambdaFunction(uploadImagesLambda));

    // DynamoDB
    const tableName = "HemnetProperties";

    const dynamoDbTable = new dynamodb.Table(this, tableName, {
      tableName,
      partitionKey: { name: "PropertyId", type: dynamodb.AttributeType.STRING },
    });

    dynamoDbTable.grant(
      uploadImagesLambda,
      "dynamodb:BatchGetItem",
      "dynamodb:PutItem"
    );

    // S3
    const bucketName = "hemnet-property-images";

    const bucket = new s3.Bucket(this, bucketName, { bucketName });

    bucket.grantWrite(uploadImagesLambda);
  }
}
