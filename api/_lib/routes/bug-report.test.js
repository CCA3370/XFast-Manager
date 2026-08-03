import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import handler, { createErrorFingerprint } from './bug-report.js'

function makeResponse() {
  return {
    statusCode: 200,
    payload: undefined,
    status(code) {
      this.statusCode = code
      return this
    },
    json(payload) {
      this.payload = payload
      return this
    },
  }
}

function makeRequest(body) {
  return {
    method: 'POST',
    body,
    headers: { host: 'example.test' },
  }
}

describe('bug report route', () => {
  beforeEach(() => {
    process.env.XFAST_GITHUB_TOKEN = 'test-token'
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    delete process.env.XFAST_GITHUB_TOKEN
  })

  it('rejects operational errors before contacting GitHub', async () => {
    const fetchMock = vi.fn()
    vi.stubGlobal('fetch', fetchMock)
    const response = makeResponse()

    await handler(
      makeRequest({ errorMessage: 'Error performing inpage operation. (os error 999)' }),
      response,
    )

    expect(response.statusCode).toBe(422)
    expect(response.payload.code).toBe('bug_report_not_allowed')
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('returns an existing open issue with the same fingerprint', async () => {
    const body = {
      errorCode: 'internal',
      errorOrigin: 'application',
      errorOperation: 'install',
      errorMessage: 'Unexpected task invariant',
    }
    const fingerprint = createErrorFingerprint(body)
    const fetchMock = vi.fn().mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => [
        {
          number: 321,
          html_url: 'https://github.com/CCA3370/XFast-Manager/issues/321',
          body: `<!-- xfast-fingerprint:${fingerprint} -->`,
        },
      ],
    })
    vi.stubGlobal('fetch', fetchMock)
    const response = makeResponse()

    await handler(makeRequest(body), response)

    expect(response.statusCode).toBe(200)
    expect(response.payload).toMatchObject({ issueNumber: 321, deduplicated: true })
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('adds a fingerprint marker when creating a reportable issue', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce({ ok: true, status: 200, json: async () => [] })
      .mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: async () => ({
          number: 322,
          html_url: 'https://github.com/CCA3370/XFast-Manager/issues/322',
        }),
      })
    vi.stubGlobal('fetch', fetchMock)
    const response = makeResponse()

    await handler(
      makeRequest({
        errorCode: 'internal',
        errorOrigin: 'application',
        errorMessage: 'Unexpected task invariant',
      }),
      response,
    )

    expect(response.statusCode).toBe(200)
    expect(response.payload).toMatchObject({ issueNumber: 322, deduplicated: false })
    const createRequest = fetchMock.mock.calls[1][1]
    const payload = JSON.parse(createRequest.body)
    expect(payload.body).toMatch(/<!-- xfast-fingerprint:[0-9a-f]{64} -->/)
  })
})
