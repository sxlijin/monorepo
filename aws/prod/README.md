# aws/prod

Production AWS infrastructure for **roundcolors.com**, defined with AWS CDK (TypeScript).

## What it provisions

Two CloudFormation stacks, wired together with CDK cross-region references:

**`RoundColorsProd` — us-west-2 (Oregon):**

- **Lambda function** (`lambda/index.mjs`) that serves the site's HTML.
- **Lambda Function URL** with `AWS_IAM` auth, reachable only through CloudFront.
- **CloudFront distribution** with the Function URL as origin (Origin Access Control), HTTP/2+3, TLS 1.2_2021.
- **Route53** A/AAAA alias records for the apex and `www`.

**`RoundColorsDns` — us-east-1:**

- **ACM certificate** for `roundcolors.com` and `www.roundcolors.com` (DNS-validated).
- **Route53 public hosted zone** for `roundcolors.com`.

CloudFront can only use ACM certificates from `us-east-1`, so the certificate
is pinned there. The (global) hosted zone sits in the same stack so the
cross-stack dependency stays one-directional (`RoundColorsProd` → `RoundColorsDns`)
and avoids a cycle. The Lambda and CloudFront origin run in Oregon.

## Prerequisites

- AWS credentials with permission to deploy (e.g. `aws sso login` / exported env vars).
- The account/region bootstrapped for CDK once: `pnpm exec cdk bootstrap`.

## Usage

Run from this directory (`aws/prod`):

```bash
pnpm install        # from the repo root; this is a workspace package
pnpm synth          # emit the CloudFormation template
pnpm diff           # diff against the deployed stack
pnpm deploy         # deploy
pnpm destroy        # tear down
```

## First deploy: point the domain at Route53

Because the hosted zone is created here for a brand-new domain, the registrar
still needs to be told to use it:

1. Run `pnpm deploy`. The ACM certificate validation and Route53 records are
   created automatically within the new zone.
2. Read the `NameServers` stack output (also visible in the Route53 console).
3. Set those four nameservers as the authoritative NS records at your domain
   registrar for `roundcolors.com`.

DNS propagation can take up to ~48h. Until the registrar points at the new
zone, certificate validation will stay pending and the site won't resolve at
the custom domain (the CloudFront `*.cloudfront.net` URL works immediately).

## Updating the site content

Edit `lambda/index.mjs` and redeploy. For anything beyond a simple page,
replace the inline HTML with real routing/templating, or swap the origin for
S3 static hosting.
