import * as lambda from "aws-cdk-lib/aws-lambda";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as logs from "aws-cdk-lib/aws-logs";
import * as iam from "aws-cdk-lib/aws-iam";
import * as ses from "aws-cdk-lib/aws-ses";
import * as sesActions from "aws-cdk-lib/aws-ses-actions";
import * as sns from "aws-cdk-lib/aws-sns";
import * as snsSubscriptions from "aws-cdk-lib/aws-sns-subscriptions";
import {
  VerifySesDomain,
  VerifySesEmailAddress,
} from "@seeebiii/ses-verify-identities";
import { Duration, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";
import * as path from "path";

export class EmailImagesStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const { FROM_EMAIL_ADDRESS, TO_EMAIL_ADDRESSES, DOMAIN_PROVIDER } =
      process.env;

    if (!FROM_EMAIL_ADDRESS)
      throw "You need to specify `FROM_EMAIL_ADDRESS=email@example.com` in .env";

    if (!TO_EMAIL_ADDRESSES)
      throw "You need to specify `TO_EMAIL_ADDRESSES=email@example.com` in .env";

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
          FROM_EMAIL_ADDRESS,
          TO_EMAIL_ADDRESSES,
        },
        memorySize: 1024,
        timeout: Duration.minutes(5),
        logRetention: logs.RetentionDays.TWO_WEEKS,
      }
    );

    // Lambda policy for SES
    const sesPolicy = new iam.PolicyStatement({
      actions: ["ses:SendEmail", "ses:SendRawEmail"],
      resources: ["*"],
    });

    emailImagesLambda.addToRolePolicy(sesPolicy);

    // Verify receiving email domain
    const domainName = FROM_EMAIL_ADDRESS.split("@")[1];

    let verifySesDomainProps = {};

    if (DOMAIN_PROVIDER !== "aws_route56")
      verifySesDomainProps = {
        addTxtRecord: false,
        addMxRecord: false,
        addDkimRecords: false,
      };

    new VerifySesDomain(this, "VerifyDomainForSes", {
      domainName,
      ...verifySesDomainProps,
    });

    // Verify email addresses
    TO_EMAIL_ADDRESSES.split(",").forEach(
      (emailAddress) =>
        new VerifySesEmailAddress(
          this,
          `VerifySesEmailAddress-${emailAddress}`,
          {
            emailAddress,
          }
        )
    );

    // SNS topic
    const topic = new sns.Topic(this, "Topic");

    // Email rule to publish the emails to SNS topic
    new ses.ReceiptRuleSet(this, "RuleSet", {
      rules: [
        {
          recipients: [FROM_EMAIL_ADDRESS],
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

    // DynamoDB
    const tableName = "HemnetProperties";

    // Grant GetItem for DynamoDb
    dynamodb.Table.fromTableName(this, tableName, tableName).grant(
      emailImagesLambda,
      "dynamodb:GetItem"
    );
  }
}
