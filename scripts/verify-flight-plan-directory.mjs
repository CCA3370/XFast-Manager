// Read-only audit of an installation's actual flight plan files.
import fs from 'node:fs/promises'
import path from 'node:path'
import crypto from 'node:crypto'
import { Window } from 'happy-dom'
import { parsePlan } from '../src/services/flightPlanDetails.ts'
const auditWindow = new Window()
globalThis.DOMParser = auditWindow.DOMParser
const directory = process.argv[2]
if (!directory) throw new Error('Pass the Output/FMS plans directory')
const report = { files: 0, formats: {}, points: 0, issues: [], changed: [] }
for (const name of await fs.readdir(directory)) {
 if(!/\.(fms|xml)$/i.test(name))continue
 const file=path.join(directory,name)
 if(!(await fs.stat(file)).isFile())continue
 const bytes=await fs.readFile(file)
 const hash=data=>crypto.createHash('sha256').update(data).digest('hex')
 const plan=parsePlan(bytes.toString('utf8'))
 const format=plan.version?`FMS ${plan.version}`:'SimBrief OFP XML'
 report.files++;report.formats[format]=(report.formats[format]??0)+1;report.points+=plan.points.length
 if(plan.issues.length)report.issues.push({name,issues:plan.issues})
 if(hash(bytes)!==hash(await fs.readFile(file)))report.changed.push(name)
}
console.log(JSON.stringify(report,null,2))
if(report.issues.length || report.changed.length) process.exitCode=1

await auditWindow.happyDOM.close()
