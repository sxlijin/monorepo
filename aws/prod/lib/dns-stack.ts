import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as route53 from 'aws-cdk-lib/aws-route53';

export interface RoundColorsDnsStackProps extends cdk.StackProps {
  /** Apex domain for the site, e.g. "roundcolors.com". */
  readonly domainName: string;
}

/**
 * The us-east-1 half of roundcolors.com.
 *
 * CloudFront can only use ACM certificates from us-east-1, so the certificate
 * lives here. The hosted zone lives here too (not in the site stack) so that
 * the certificate's DNS validation and the zone are in one stack — keeping the
 * cross-stack dependency one-directional (site -> dns) and avoiding a cycle.
 * Route53 is a global service, so records added from the us-west-2 site stack
 * still land in this zone.
 */
export class RoundColorsDnsStack extends cdk.Stack {
  readonly hostedZone: route53.IHostedZone;
  readonly certificate: acm.ICertificate;

  constructor(scope: Construct, id: string, props: RoundColorsDnsStackProps) {
    super(scope, id, props);

    const { domainName } = props;

    const hostedZone = new route53.PublicHostedZone(this, 'HostedZone', {
      zoneName: domainName,
    });

    const certificate = new acm.Certificate(this, 'Certificate', {
      domainName,
      subjectAlternativeNames: [`www.${domainName}`],
      validation: acm.CertificateValidation.fromDns(hostedZone),
    });

    this.hostedZone = hostedZone;
    this.certificate = certificate;

    new cdk.CfnOutput(this, 'NameServers', {
      description: 'Set these nameservers at your domain registrar',
      value: cdk.Fn.join(', ', hostedZone.hostedZoneNameServers ?? []),
    });
  }
}
