import type {DeviceTransport} from "../../packages/device-protocol/src/index.ts";
export interface DeviceService extends DeviceTransport{}
export interface HostAutomation{executeMacro(id:string):Promise<{ok:boolean;message:string}>}
export interface PersistentConfig{load():Promise<string|null>;save(serialized:string):Promise<void>}
