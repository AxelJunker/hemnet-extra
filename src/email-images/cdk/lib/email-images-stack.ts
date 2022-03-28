import * as lambda from "aws-cdk-lib/aws-lambda";
import * as ses from "aws-cdk-lib/aws-ses";
import * as sesActions from "aws-cdk-lib/aws-ses-actions";
import * as sns from "aws-cdk-lib/aws-sns";
import * as snsSubscriptions from "aws-cdk-lib/aws-sns-subscriptions";
import { VerifySesDomain } from "@seeebiii/ses-verify-identities";
import { Duration, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";
import * as path from "path";

export class EmailImagesStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    if (!process.env.EMAIL_ADDRESS) {
      throw "You need to specify `EMAIL_ADDRESS=email@example.com` in aws.env";
    }

    // Lambda
    const emailImagesLambda = new lambda.DockerImageFunction(
      this,
      "HemnetEmailImagesHandler",
      {
        functionName: "hemnet-email-images",
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

    const emailAddress = process.env.EMAIL_ADDRESS;
    const domainName = emailAddress.split("@")[1];

    let verifySesDomainProps = {};

    if (process.env.DOMAIN_PROVIDER !== "aws_route56") {
      verifySesDomainProps = {
        addTxtRecord: false,
        addMxRecord: false,
        addDkimRecords: false,
      };
    }

    // Verify domain
    new VerifySesDomain(this, "SesDomainVerification", {
      domainName,
      ...verifySesDomainProps,
    });

    // SNS topic
    const topic = new sns.Topic(this, "Topic");

    // Email rule to publish the emails to SNS topic
    new ses.ReceiptRuleSet(this, "RuleSet", {
      rules: [
        {
          recipients: [emailAddress],
          actions: [
            new sesActions.Sns({
              topic,
            }),
          ],
          enabled: true,
        },
      ],
    });

    // Subscribe Lambda to SNS topic
    topic.addSubscription(
      new snsSubscriptions.LambdaSubscription(emailImagesLambda)
    );
  }
}
