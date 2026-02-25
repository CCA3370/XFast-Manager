const body = JSON.stringify({
  errorTitle: "[TEST] API field test",
  errorMessage: "Testing issueNumber field in response",
  appVersion: "test",
  os: "test",
  arch: "test",
  logs: "",
  category: "Other"
});

const res = await fetch("https://x-fast-manager.vercel.app/api/bug-report", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body
});

const text = await res.text();
console.log("Status:", res.status);
console.log("Response body:", text);
