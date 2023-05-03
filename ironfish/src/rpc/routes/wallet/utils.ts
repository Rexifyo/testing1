/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
import { IronfishNode } from '../../../node'
import { Note } from '../../../primitives'
import { CurrencyUtils } from '../../../utils'
import { Account } from '../../../wallet'
import { DecryptedNoteValue } from '../../../wallet/walletdb/decryptedNoteValue'
import { TransactionValue } from '../../../wallet/walletdb/transactionValue'
import { ValidationError } from '../../adapters'
import {
  RcpAccountAssetBalanceDelta,
  RpcAccountDecryptedNote,
  RpcAccountTransaction,
} from './types'

export function getAccount(node: IronfishNode, name?: string): Account {
  if (name) {
    const account = node.wallet.getAccountByName(name)
    if (account) {
      return account
    }
    throw new ValidationError(`No account with name ${name}`)
  }

  const defaultAccount = node.wallet.getDefaultAccount()
  if (defaultAccount) {
    return defaultAccount
  }

  throw new ValidationError(
    `No account is currently active.\n\n` +
      `Use ironfish wallet:create <name> to first create an account`,
  )
}

export function serializeRpcAccountTransaction(
  transaction: TransactionValue,
): RpcAccountTransaction {
  return {
    hash: transaction.transaction.hash().toString('hex'),
    fee: transaction.transaction.fee().toString(),
    blockHash: transaction.blockHash?.toString('hex'),
    blockSequence: transaction.sequence ?? undefined,
    notesCount: transaction.transaction.notes.length,
    spendsCount: transaction.transaction.spends.length,
    mintsCount: transaction.transaction.mints.length,
    burnsCount: transaction.transaction.burns.length,
    expiration: transaction.transaction.expiration(),
    timestamp: transaction.timestamp.getTime(),
    submittedSequence: transaction.submittedSequence,
  }
}

export async function getAssetBalanceDeltas(
  node: IronfishNode,
  transaction: TransactionValue,
): Promise<RcpAccountAssetBalanceDelta[]> {
  const assetBalanceDeltas = new Array<RcpAccountAssetBalanceDelta>()

  for (const [assetId, delta] of transaction.assetBalanceDeltas.entries()) {
    // TODO: update to use wallet assets store
    const asset = await node.chain.getAssetById(assetId)

    const assetName = asset?.name.toString('hex') ?? ''

    assetBalanceDeltas.push({
      assetId: assetId.toString('hex'),
      assetName,
      delta: delta.toString(),
    })
  }

  return assetBalanceDeltas
}

export async function getTransactionNotes(
  node: IronfishNode,
  account: Account,
  transaction: TransactionValue,
): Promise<Array<DecryptedNoteValue>> {
  const notes = []
  const decryptNotesPayloads = []

  let accountHasSpend = false
  for (const spend of transaction.transaction.spends) {
    const noteHash = await account.getNoteHash(spend.nullifier)

    if (noteHash !== undefined) {
      accountHasSpend = true
      break
    }
  }

  for (const note of transaction.transaction.notes) {
    const decryptedNote = await account.getDecryptedNote(note.hash())

    if (decryptedNote) {
      notes.push(decryptedNote)
      continue
    }

    decryptNotesPayloads.push({
      serializedNote: note.serialize(),
      incomingViewKey: account.incomingViewKey,
      outgoingViewKey: account.outgoingViewKey,
      viewKey: account.viewKey,
      currentNoteIndex: null,
      decryptForSpender: true,
    })
  }

  if (accountHasSpend && decryptNotesPayloads.length > 0) {
    const decryptedSends = await node.workerPool.decryptNotes(decryptNotesPayloads)

    for (const note of decryptedSends) {
      if (note === null) {
        continue
      }

      notes.push({
        accountId: '',
        note: new Note(note.serializedNote),
        index: null,
        nullifier: null,
        transactionHash: transaction.transaction.hash(),
        spent: false,
        blockHash: transaction.blockHash,
        sequence: transaction.sequence,
      })
    }
  }

  return notes
}

export async function getAccountDecryptedNotes(
  node: IronfishNode,
  account: Account,
  transaction: TransactionValue,
): Promise<RpcAccountDecryptedNote[]> {
  const notes = await getTransactionNotes(node, account, transaction)

  const serializedNotes: RpcAccountDecryptedNote[] = []
  for await (const decryptedNote of notes) {
    const isOwner = decryptedNote.note.owner() === account.publicAddress
    const asset = await node.chain.getAssetById(decryptedNote.note.assetId())
    const note = decryptedNote.note

    serializedNotes.push({
      isOwner,
      owner: note.owner(),
      memo: note.memo(),
      value: CurrencyUtils.encode(note.value()),
      assetId: note.assetId().toString('hex'),
      assetName: asset?.name.toString('hex') || '',
      sender: note.sender(),
      spent: decryptedNote.spent,
      hash: note.hash().toString('hex'),
    })
  }

  return serializedNotes
}
