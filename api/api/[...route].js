import bugReportHandler from '../bug-report.js'
import feedbackIssueHandler from '../feedback-issue.js'
import issueCommentHandler from '../issue-comment.js'
import issueDetailHandler from '../issue-detail.js'
import issueDraftHandler from '../issue-draft.js'
import issueRedirectHandler from '../issue-redirect.js'
import issueUpdatesHandler from '../issue-updates.js'
import latestJsonHandler from '../latest-json.js'
import libraryLinkHandler from '../library-link.js'
import libraryLinksDataHandler from '../library-links-data.js'
import liveryPatternsDataHandler from '../livery-patterns-data.js'
import releaseRedirectHandler from '../release-redirect.js'
import updateReleaseHandler from '../update-release.js'

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
  'release-redirect': releaseRedirectHandler,
  'update-release': updateReleaseHandler,
}

function getRouteSegments(req) {
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
