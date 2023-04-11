/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
import { PUBLIC_ADDRESS_LENGTH } from '@ironfish/rust-nodejs'
import bufio from 'bufio'
import { IDatabase, IDatabaseEncoding, IDatabaseStore, StringEncoding } from '../../../storage'

const KEY_LENGTH = 32
const VIEW_KEY_LENGTH = 64
const VERSION_LENGTH = 2

export interface AccountValue {
  version: number
  id: string
  name: string
  spendingKey: Buffer
  viewKey: Buffer
  incomingViewKey: Buffer
  outgoingViewKey: Buffer
  publicAddress: Buffer
}

export class AccountValueEncoding implements IDatabaseEncoding<AccountValue> {
  serialize(value: AccountValue): Buffer {
    const bw = bufio.write(this.getSize(value))
    bw.writeU16(value.version)
    bw.writeVarString(value.id, 'utf8')
    bw.writeVarString(value.name, 'utf8')
    bw.writeBytes(value.spendingKey)
    bw.writeBytes(value.viewKey)
    bw.writeBytes(value.incomingViewKey)
    bw.writeBytes(value.outgoingViewKey)
    bw.writeBytes(value.publicAddress)
    return bw.render()
  }

  deserialize(buffer: Buffer): AccountValue {
    const reader = bufio.read(buffer, true)
    const version = reader.readU16()
    const id = reader.readVarString('utf8')
    const name = reader.readVarString('utf8')
    const spendingKey = reader.readBytes(KEY_LENGTH)
    const viewKey = reader.readBytes(VIEW_KEY_LENGTH)
    const incomingViewKey = reader.readBytes(KEY_LENGTH)
    const outgoingViewKey = reader.readBytes(KEY_LENGTH)
    const publicAddress = reader.readBytes(PUBLIC_ADDRESS_LENGTH)

    return {
      version,
      id,
      name,
      spendingKey,
      viewKey,
      incomingViewKey,
      outgoingViewKey,
      publicAddress,
    }
  }

  getSize(value: AccountValue): number {
    let size = 0
    size += VERSION_LENGTH
    size += bufio.sizeVarString(value.id, 'utf8')
    size += bufio.sizeVarString(value.name, 'utf8')
    size += KEY_LENGTH
    size += VIEW_KEY_LENGTH
    size += KEY_LENGTH
    size += KEY_LENGTH
    size += PUBLIC_ADDRESS_LENGTH

    return size
  }
}

export function GetNewStores(db: IDatabase): {
  accounts: IDatabaseStore<{ key: string; value: AccountValue }>
} {
  const accounts: IDatabaseStore<{ key: string; value: AccountValue }> = db.addStore(
    {
      name: 'a',
      keyEncoding: new StringEncoding(),
      valueEncoding: new AccountValueEncoding(),
    },
    false,
  )

  return { accounts }
}
