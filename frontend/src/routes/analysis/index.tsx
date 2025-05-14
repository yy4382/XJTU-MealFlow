import { createFileRoute, Navigate } from '@tanstack/react-router'

export const Route = createFileRoute('/analysis/')({
  component: AnalysisIndexPage,
})

function AnalysisIndexPage() {
  return <Navigate to="/analysis/time-period" />
}
