import * as lambda from "aws-cdk-lib/aws-lambda";
import * as logs from "aws-cdk-lib/aws-logs";
import * as targets from "aws-cdk-lib/aws-events-targets";
import * as events from "aws-cdk-lib/aws-events";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as cdk from "aws-cdk-lib";
import { Duration, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";
import * as path from "path";

export class UploadImagesStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const { SUBSCRIPTION_ID } = process.env;

    if (!SUBSCRIPTION_ID)
      throw "You need to specify `SUBSCRIPTION_ID=123456` in .env";

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
          SUBSCRIPTION_ID,
        },
        memorySize: 1024,
        timeout: Duration.minutes(5),
        logRetention: logs.RetentionDays.TWO_WEEKS,
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

    const lifecycleRule: s3.LifecycleRule = {
      enabled: true,
      expiration: cdk.Duration.days(120),
    };

    bucket.addLifecycleRule(lifecycleRule);
  }
}
