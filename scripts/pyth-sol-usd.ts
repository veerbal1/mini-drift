/// <reference lib="es2020.bigint" />
/// <reference types="node" />

import { Connection, PublicKey, clusterApiUrl } from "@solana/web3.js";

const CLUSTER = "devnet";
const RPC_URL = clusterApiUrl(CLUSTER);
const PYTH_SOL_USD_PRICE_ACCOUNT =
  process.env.PYTH_SOL_USD_PRICE_ACCOUNT ??
  "7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE";

type DecodedPythPrice = {
  feedId: string;
  price: bigint;
  confidence: bigint;
  exponent: number;
  publishTime: bigint;
  previousPublishTime: bigint;
  emaPrice: bigint;
  emaConfidence: bigint;
  postedSlot: bigint;
  verificationLevel: string;
};

function scalePythNumber(value: bigint, exponent: number): string {
  const negative = value < BigInt(0);
  const absolute = negative ? -value : value;

  if (exponent >= 0) {
    return `${negative ? "-" : ""}${absolute}${"0".repeat(exponent)}`;
  }

  const decimals = -exponent;
  const absoluteString = absolute.toString();
  const raw =
    absoluteString.length > decimals
      ? absoluteString
      : `${"0".repeat(decimals + 1 - absoluteString.length)}${absoluteString}`;
  const whole = raw.slice(0, -decimals);
  const fraction = raw.slice(-decimals).replace(/0+$/, "");

  return `${negative ? "-" : ""}${whole}${fraction ? `.${fraction}` : ""}`;
}

function decodePythPriceUpdateV2(data: Buffer): DecodedPythPrice {
  let offset = 8; // Anchor account discriminator
  offset += 32; // write_authority

  const verificationVariant = data.readUInt8(offset);
  let verificationLevel: string;
  if (verificationVariant === 0) {
    const signatures = data.readUInt8(offset + 1);
    verificationLevel = `partial(${signatures})`;
    offset += 2;
  } else if (verificationVariant === 1) {
    verificationLevel = "full";
    offset += 1;
  } else {
    throw new Error(
      `Unknown verification level variant: ${verificationVariant}`
    );
  }

  const feedId = data.subarray(offset, offset + 32).toString("hex");
  offset += 32;

  const price = data.readBigInt64LE(offset);
  offset += 8;

  const confidence = data.readBigUInt64LE(offset);
  offset += 8;

  const exponent = data.readInt32LE(offset);
  offset += 4;

  const publishTime = data.readBigInt64LE(offset);
  offset += 8;

  const previousPublishTime = data.readBigInt64LE(offset);
  offset += 8;

  const emaPrice = data.readBigInt64LE(offset);
  offset += 8;

  const emaConfidence = data.readBigUInt64LE(offset);
  offset += 8;

  const postedSlot = data.readBigUInt64LE(offset);

  return {
    feedId,
    price,
    confidence,
    exponent,
    publishTime,
    previousPublishTime,
    emaPrice,
    emaConfidence,
    postedSlot,
    verificationLevel,
  };
}

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");
  const priceAccount = new PublicKey(PYTH_SOL_USD_PRICE_ACCOUNT);
  const response = await connection.getAccountInfoAndContext(priceAccount, {
    commitment: "confirmed",
  });

  if (!response.value) {
    throw new Error(`Pyth SOL/USD price account not found: ${priceAccount}`);
  }

  const decoded = decodePythPriceUpdateV2(response.value.data);

  console.log(`cluster: ${CLUSTER}`);
  console.log(`priceAccount: ${priceAccount.toBase58()}`);
  console.log(`price: ${scalePythNumber(decoded.price, decoded.exponent)}`);
  console.log(
    `confidence: ${scalePythNumber(decoded.confidence, decoded.exponent)}`
  );
  console.log(`exponent: ${decoded.exponent}`);
  console.log(`rawPrice: ${decoded.price.toString()}`);
  console.log(`rawConfidence: ${decoded.confidence.toString()}`);
  console.log(`feedId: 0x${decoded.feedId}`);
  console.log(`verificationLevel: ${decoded.verificationLevel}`);
  console.log(`contextSlot: ${response.context.slot}`);
  console.log(`postedSlot: ${decoded.postedSlot.toString()}`);
  console.log(`publishTime: ${decoded.publishTime.toString()}`);
  console.log(`previousPublishTime: ${decoded.previousPublishTime.toString()}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
