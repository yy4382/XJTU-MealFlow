import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings')({
  component: SettingsPage,
});

function SettingsPage() {
  // TODO: Implement settings form (e.g., for account, hallticket)
  return (
    <div className="p-4">
      <h1 className="text-2xl font-semibold mb-4">Settings</h1>
      <p>Application settings will be configured here.</p>
      {/* Form for API key, theme, etc. */}
    </div>
  );
} 