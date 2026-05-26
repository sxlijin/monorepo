import * as path from 'path';
import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deploy from 'aws-cdk-lib/aws-s3-deployment';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as targets from 'aws-cdk-lib/aws-route53-targets';

export interface RoundcolorsSiteStackProps extends cdk.StackProps {
  /** Apex domain for the site, e.g. "roundcolors.com". */
  readonly domainName: string;
  /** Hosted zone for the domain (created in the us-east-1 DNS stack). */
  readonly hostedZone: route53.IHostedZone;
  /** CloudFront certificate (must be us-east-1; created in the DNS stack). */
  readonly certificate: acm.ICertificate;
}

/**
 * The us-west-2 (Oregon) half of roundcolors.com: a private S3 bucket of static
 * content fronted by CloudFront (Origin Access Control), plus the Route53 alias
 * records. Consumes the certificate and hosted zone from the DNS stack via CDK
 * cross-region references.
 *
 * Content layout in the bucket mirrors the URL path, e.g. the page served at
 * `roundcolors.com/barcodes` lives at the `barcodes/index.html` key.
 */
export class RoundcolorsSiteStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: RoundcolorsSiteStackProps) {
    super(scope, id, props);

    const { domainName, hostedZone, certificate } = props;
    const wwwDomainName = `www.${domainName}`;

    // Private bucket — only CloudFront reads it, via OAC (set up below).
    const bucket = new s3.Bucket(this, 'SiteBucket', {
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      enforceSSL: true,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });

    // Upload barcodes-app/site/index.html to the `barcodes/` prefix, so
    // it's served at roundcolors.com/barcodes.
    new s3deploy.BucketDeployment(this, 'DeployBarcodes', {
      sources: [
        s3deploy.Source.asset(
          path.join(__dirname, '..', '..', '..', 'barcodes-app', 'site'),
        ),
      ],
      destinationBucket: bucket,
      destinationKeyPrefix: 'barcodes',
    });

    // S3 with OAC serves exact keys only — it has no notion of directory index
    // documents below the root. This viewer-request function maps directory-ish
    // URLs to their index.html, e.g. /barcodes -> /barcodes/index.html.
    const indexRewrite = new cloudfront.Function(this, 'IndexRewrite', {
      runtime: cloudfront.FunctionRuntime.JS_2_0,
      code: cloudfront.FunctionCode.fromInline(`
function handler(event) {
  var request = event.request;
  var uri = request.uri;
  if (uri.endsWith('/')) {
    request.uri = uri + 'index.html';
  } else if (!uri.split('/').pop().includes('.')) {
    request.uri = uri + '/index.html';
  }
  return request;
}
      `),
    });

    const distribution = new cloudfront.Distribution(this, 'Distribution', {
      comment: `roundcolors.com (${domainName})`,
      defaultBehavior: {
        origin: origins.S3BucketOrigin.withOriginAccessControl(bucket),
        viewerProtocolPolicy:
          cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        // No caching for now, per request — every request hits the origin.
        cachePolicy: cloudfront.CachePolicy.CACHING_DISABLED,
        functionAssociations: [
          {
            function: indexRewrite,
            eventType: cloudfront.FunctionEventType.VIEWER_REQUEST,
          },
        ],
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
    new cdk.CfnOutput(this, 'BarcodesUrl', {
      value: `https://${domainName}/barcodes`,
    });
  }
}
