import bugReportHandler from './routes/bug-report.js'
import feedbackIssueHandler from './routes/feedback-issue.js'
import issueCommentHandler from './routes/issue-comment.js'
import issueDetailHandler from './routes/issue-detail.js'
import issueDraftHandler from './routes/issue-draft.js'
import issueRedirectHandler from './routes/issue-redirect.js'
import issueUpdatesHandler from './routes/issue-updates.js'
import latestJsonHandler from './routes/latest-json.js'
import libraryLinkHandler from './routes/library-link.js'
import libraryLinksDataHandler from './routes/library-links-data.js'
import liveryPatternsDataHandler from './routes/livery-patterns-data.js'
import outputCleanupItemSubmissionHandler from './routes/output-cleanup-item-submission.js'
import outputCleanupItemsDataHandler from './routes/output-cleanup-items-data.js'
import releaseRedirectHandler from './routes/release-redirect.js'
import updateReleaseHandler from './routes/update-release.js'

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

export default async function routeDispatcher(req, res) {
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
