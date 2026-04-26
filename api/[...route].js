import bugReportHandler from './_lib/routes/bug-report.js'
import feedbackIssueHandler from './_lib/routes/feedback-issue.js'
import issueCommentHandler from './_lib/routes/issue-comment.js'
import issueDetailHandler from './_lib/routes/issue-detail.js'
import issueDraftHandler from './_lib/routes/issue-draft.js'
import issueRedirectHandler from './_lib/routes/issue-redirect.js'
import issueUpdatesHandler from './_lib/routes/issue-updates.js'
import latestJsonHandler from './_lib/routes/latest-json.js'
import libraryLinkHandler from './_lib/routes/library-link.js'
import libraryLinksDataHandler from './_lib/routes/library-links-data.js'
import liveryPatternsDataHandler from './_lib/routes/livery-patterns-data.js'
import outputCleanupItemSubmissionHandler from './_lib/routes/output-cleanup-item-submission.js'
import outputCleanupItemsDataHandler from './_lib/routes/output-cleanup-items-data.js'
import releaseRedirectHandler from './_lib/routes/release-redirect.js'
import updateReleaseHandler from './_lib/routes/update-release.js'

const handlers = {
  'bug-report': bugReportHandler,
  'feedback-issue': feedbackIssueHandler,
  'issue-comment': issueCommentHandler,
  'issue-detail': issueDetailHandler,
  'issue-draft': issueDraftHandler,
  'issue-redirect': issueRedirectHandler,
  'issue-updates': issueUpdatesHandler,
  'latest-json': latestJsonHandler,
  'library-link': libraryLinkHandler,
  'library-links-data': libraryLinksDataHandler,
  'livery-patterns-data': liveryPatternsDataHandler,
  'output-cleanup-item-submission': outputCleanupItemSubmissionHandler,
  'output-cleanup-items-data': outputCleanupItemsDataHandler,
  'release-redirect': releaseRedirectHandler,
  'update-release': updateReleaseHandler,
}

function getRouteSegments(req) {
  if (typeof req.url === 'string' && req.url.length > 0) {
    const pathname = new URL(req.url, 'https://x-fast-manager.vercel.app').pathname
    const segments = pathname.split('/').filter(Boolean)

    if (segments[0] === 'api') {
      return segments.slice(1)
    }

    return segments
  }

  const route = req.query?.route

  if (Array.isArray(route)) {
    return route
  }

  if (typeof route === 'string' && route.length > 0) {
    return [route]
  }

  return []
}

export default async function handler(req, res) {
  const segments = getRouteSegments(req)
  if (segments.length !== 1) {
    return res.status(404).json({ error: 'Not found' })
  }

  const target = handlers[segments[0]]
  if (!target) {
    return res.status(404).json({ error: 'Not found' })
  }

  if (req.query && Object.prototype.hasOwnProperty.call(req.query, 'route')) {
    delete req.query.route
  }

  return target(req, res)
}
