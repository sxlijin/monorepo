#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { RoundcolorsDnsStack } from '../lib/dns-stack';
import { RoundcolorsSiteStack } from '../lib/site-stack';

const app = new cdk.App();

const domainName = 'roundcolors.com';
const account = process.env.CDK_DEFAULT_ACCOUNT;

// CloudFront certificates must live in us-east-1, so the certificate and the
// (global) hosted zone are deployed there. Everything else runs in Oregon.
const dns = new RoundcolorsDnsStack(app, 'RoundcolorsDns', {
  domainName,
  env: { account, region: 'us-east-1' },
  crossRegionReferences: true,
  description: 'roundcolors.com hosted zone + CloudFront certificate (us-east-1)',
});

new RoundcolorsSiteStack(app, 'RoundcolorsProd', {
  domainName,
  hostedZone: dns.hostedZone,
  certificate: dns.certificate,
  env: { account, region: 'us-west-2' },
  crossRegionReferences: true,
  description: 'roundcolors.com Lambda + CloudFront (us-west-2 / Oregon)',
});
