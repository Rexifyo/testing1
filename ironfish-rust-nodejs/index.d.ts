/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export class ExternalObject<T> {
  readonly '': {
    readonly '': unique symbol
    [K: symbol]: T
  }
}
export const KEY_LENGTH: number
export const NONCE_LENGTH: number
export function randomBytes(bytesLength: number): Uint8Array
export interface BoxedMessage {
  nonce: string
  boxedMessage: string
}
export function boxMessage(plaintext: string, senderSecretKey: Uint8Array, recipientPublicKey: string): BoxedMessage
export function unboxMessage(boxedMessage: string, nonce: string, senderPublicKey: string, recipientSecretKey: Uint8Array): string
export interface NativeSpendProof {
  treeSize: number
  rootHash: Buffer
  nullifier: Buffer
}
export function verifyTransactions(serializedTransactions: Array<Buffer>): boolean
export interface Key {
  spending_key: string
  incoming_view_key: string
  outgoing_view_key: string
  public_address: string
}
export function generateKey(): Key
export function generateNewPublicAddress(privateKey: string): Key
export function initializeSapling(): void
export function isValidPublicAddress(hexAddress: string): boolean
export class BoxKeyPair {
  constructor()
  static fromHex(secretHex: string): BoxKeyPair
  get publicKey(): Buffer
  get secretKey(): Buffer
}
export type NativeRollingFilter = RollingFilter
export class RollingFilter {
  constructor(items: number, rate: number)
  add(value: Buffer): void
  test(value: Buffer): boolean
}
export type NativeNoteEncrypted = NoteEncrypted
export class NoteEncrypted {
  constructor(jsBytes: Buffer)
  serialize(): Buffer
  equals(other: NoteEncrypted): boolean
  merkleHash(): Buffer
  /**
   * Hash two child hashes together to calculate the hash of the
   * new parent
   */
  static combineHash(depth: number, jsLeft: Buffer, jsRight: Buffer): Buffer
  /** Returns undefined if the note was unable to be decrypted with the given key. */
  decryptNoteForOwner(incomingHexKey: string): Buffer | null
  /** Returns undefined if the note was unable to be decrypted with the given key. */
  decryptNoteForSpender(outgoingHexKey: string): Buffer | null
}
export type NativeNote = Note
export class Note {
  constructor(owner: string, value: bigint, memo: string)
  static deserialize(jsBytes: Buffer): NativeNote
  serialize(): Buffer
  /** Value this note represents. */
  value(): bigint
  /**
   * Arbitrary note the spender can supply when constructing a spend so the
   * receiver has some record from whence it came.
   * Note: While this is encrypted with the output, it is not encoded into
   * the proof in any way.
   */
  memo(): string
  /**
   * Compute the nullifier for this note, given the private key of its owner.
   *
   * The nullifier is a series of bytes that is published by the note owner
   * only at the time the note is spent. This key is collected in a massive
   * 'nullifier set', preventing double-spend.
   */
  nullifier(ownerPrivateKey: string, position: bigint): Buffer
}
export type NativeTransactionPosted = TransactionPosted
export class TransactionPosted {
  constructor(jsBytes: Buffer)
  serialize(): Buffer
  verify(): boolean
  notesLength(): number
  getNote(index: number): Buffer
  spendsLength(): number
  getSpend(index: number): NativeSpendProof
  fee(): bigint
  transactionSignature(): Buffer
  hash(): Buffer
  expirationSequence(): number
}
export type NativeTransaction = Transaction
export class Transaction {
  constructor(spenderHexKey: string)
  /** Create a proof of a new note owned by the recipient in this transaction. */
  receive(note: Note): string
  /** Spend the note owned by spender_hex_key at the given witness location. */
  spend(note: Note, witness: object): string
  /**
   * Special case for posting a miners fee transaction. Miner fee transactions
   * are unique in that they generate currency. They do not have any spends
   * or change and therefore have a negative transaction fee. In normal use,
   * a miner would not accept such a transaction unless it was explicitly set
   * as the miners fee.
   */
  post_miners_fee(): Buffer
  /**
   * Post the transaction. This performs a bit of validation, and signs
   * the spends with a signature that proves the spends are part of this
   * transaction.
   *
   * Transaction fee is the amount the spender wants to send to the miner
   * for mining this transaction. This has to be non-negative; sane miners
   * wouldn't accept a transaction that takes money away from them.
   *
   * sum(spends) - sum(outputs) - intended_transaction_fee - change = 0
   * aka: self.transaction_fee - intended_transaction_fee - change = 0
   */
  post(changeGoesTo: string | undefined | null, intendedTransactionFee: bigint): Buffer
  setExpirationSequence(expirationSequence: number): void
}
export class FoundBlockResult {
  randomness: string
  miningRequestId: number
  constructor(randomness: string, miningRequestId: number)
}
export class ThreadPoolHandler {
  constructor(threadCount: number, batchSize: number)
  newWork(headerBytes: Buffer, target: Buffer, miningRequestId: number): void
  stop(): void
  pause(): void
  getFoundBlock(): FoundBlockResult | null
  getHashRateSubmission(): number
}
