import * as path from 'path';
import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as targets from 'aws-cdk-lib/aws-route53-targets';

export interface RoundColorsSiteStackProps extends cdk.StackProps {
  /** Apex domain for the site, e.g. "roundcolors.com". */
  readonly domainName: string;
  /** Hosted zone for the domain (created in the us-east-1 DNS stack). */
  readonly hostedZone: route53.IHostedZone;
  /** CloudFront certificate (must be us-east-1; created in the DNS stack). */
  readonly certificate: acm.ICertificate;
}

/**
 * The us-west-2 (Oregon) half of roundcolors.com: the Lambda that serves the
 * site, its Function URL, the CloudFront distribution, and the Route53 alias
 * records. Consumes the certificate and hosted zone from the DNS stack via
 * CDK cross-region references.
 */
export class RoundColorsSiteStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: RoundColorsSiteStackProps) {
    super(scope, id, props);

    const { domainName, hostedZone, certificate } = props;
    const wwwDomainName = `www.${domainName}`;

    // The application: a single Lambda returning the site's HTML.
    const fn = new lambda.Function(this, 'SiteFunction', {
      runtime: lambda.Runtime.NODEJS_20_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset(path.join(__dirname, '..', 'lambda')),
      memorySize: 128,
      timeout: cdk.Duration.seconds(10),
      architecture: lambda.Architecture.ARM_64,
    });

    // Function URL fronted by CloudFront. AWS_IAM auth + Origin Access Control
    // means only CloudFront can invoke the URL, not the public internet.
    const fnUrl = fn.addFunctionUrl({
      authType: lambda.FunctionUrlAuthType.AWS_IAM,
    });

    const distribution = new cloudfront.Distribution(this, 'Distribution', {
      comment: `roundcolors.com (${domainName})`,
      defaultBehavior: {
        origin: origins.FunctionUrlOrigin.withOriginAccessControl(fnUrl),
        viewerProtocolPolicy:
          cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
      },
      domainNames: [domainName, wwwDomainName],
      certificate,
      minimumProtocolVersion: cloudfront.SecurityPolicyProtocol.TLS_V1_2_2021,
      httpVersion: cloudfront.HttpVersion.HTTP2_AND_3,
    });

    // Alias both the apex and www at the CloudFront distribution (A + AAAA).
    const aliasTarget = route53.RecordTarget.fromAlias(
      new targets.CloudFrontTarget(distribution),
    );
    for (const name of ['', 'www']) {
      const recordName = name || undefined;
      new route53.ARecord(this, `ARecord${name || 'Apex'}`, {
        zone: hostedZone,
        recordName,
        target: aliasTarget,
      });
      new route53.AaaaRecord(this, `AaaaRecord${name || 'Apex'}`, {
        zone: hostedZone,
        recordName,
        target: aliasTarget,
      });
    }

    new cdk.CfnOutput(this, 'DistributionDomainName', {
      value: distribution.distributionDomainName,
    });
    new cdk.CfnOutput(this, 'SiteUrl', {
      value: `https://${domainName}`,
    });
  }
}
