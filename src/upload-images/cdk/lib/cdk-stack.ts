// import { Stack, StackProps } from "aws-cdk-lib";
// import { Construct } from "constructs";
import * as cdk from "@aws-cdk/core";
import * as lambda from "@aws-cdk/aws-lambda";
import * as targets from "@aws-cdk/aws-events-targets";
import * as events from "@aws-cdk/aws-events";

// export class CdkStack extends Stack {
//   constructor(scope: Construct, id: string, props?: StackProps) {
//     super(scope, id, props);
//
//     // The code that defines your stack goes here
//
//     // example resource
//     // const queue = new sqs.Queue(this, 'CdkQueue', {
//     //   visibilityTimeout: cdk.Duration.seconds(300)
//     // });
//   }
// }
//
export class UploadImagesLambdaStack extends cdk.Stack {
  constructor(scope: cdk.Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const target = "x86_64-unknown-linux-musl";

    const uploadImagesLambda = new lambda.Function(
      this,
      "UploadImagesHandler",
      {
        code: lambda.Code.fromAsset("..", {
          bundling: {
            command: [
              "bash",
              "-c",
              `rustup target add ${target} && cargo build --release --target ${target} && cp target/${target}/release/upload-images /asset-output/bootstrap`,
            ],
            image: cdk.DockerImage.fromRegistry("rust:1.58-slim"),
          },
        }),
        functionName: "upload-images",
        handler: "main",
        runtime: lambda.Runtime.PROVIDED_AL2,
      }
    );

    const eventRule = new events.Rule(this, "scheduleRule", {
      schedule: events.Schedule.rate(cdk.Duration.minutes(1)),
    });

    eventRule.addTarget(new targets.LambdaFunction(uploadImagesLambda));
  }
}
