import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

export const Route = createFileRoute('/settings')({
  component: SettingsPage,
});

function SettingsPage() {
  const [account, setAccount] = useState('');
  const [hallticket, setHallticket] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    const fetchSettings = async () => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await fetch('/api/config/account-cookie');
        if (!response.ok) {
          if (response.status === 404) {
            // If not found, it's okay, just leave fields blank
            setAccount('');
            setHallticket('');
            console.info('Account and cookie not found, leaving fields blank.');
          } else {
            throw new Error(`Failed to fetch settings: ${response.statusText}`);
          }
        } else {
          const data = await response.json();
          setAccount(data.account || '');
          // Extract hallticket value from the cookie string "hallticket=value"
          const cookieValue = data.cookie || '';
          const hallticketMatch = cookieValue.match(/hallticket=([^;]*)/);
          setHallticket(hallticketMatch ? hallticketMatch[1] : '');
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'An unknown error occurred');
        console.error(err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchSettings();
  }, []);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsLoading(true);
    setError(null);
    setSuccessMessage(null);

    try {
      // Update account
      const accountRes = await fetch('/api/config/account', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ account }),
      });
      if (!accountRes.ok) {
        const errorData = await accountRes.text();
        throw new Error(`Failed to update account: ${accountRes.statusText} - ${errorData}`);
      }

      // Update hallticket
      const hallticketRes = await fetch('/api/config/hallticket', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ hallticket }),
      });
      if (!hallticketRes.ok) {
        const errorData = await hallticketRes.text();
        throw new Error(`Failed to update hallticket: ${hallticketRes.statusText} - ${errorData}`);
      }

      setSuccessMessage('Settings updated successfully!');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An unknown error occurred while saving.');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="p-4 max-w-md mx-auto">
      <div className="mb-6">
        <h1 className="text-3xl font-bold tracking-tight mb-2">Settings</h1>
        <p className="text-muted-foreground">
          Manage your account and application settings.
        </p>
      </div>

      {error && (
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative mb-4" role="alert">
          <strong className="font-bold">Error: </strong>
          <span className="block sm:inline">{error}</span>
        </div>
      )}

      {successMessage && (
        <div className="bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded relative mb-4" role="alert">
          <strong className="font-bold">Success: </strong>
          <span className="block sm:inline">{successMessage}</span>
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-6">
        <div>
          <label htmlFor="account" className="block text-sm font-medium text-gray-700">
            Account
          </label>
          <input
            type="text"
            id="account"
            value={account}
            onChange={(e) => setAccount(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
            disabled={isLoading}
          />
        </div>

        <div>
          <label htmlFor="hallticket" className="block text-sm font-medium text-gray-700">
            Hallticket
          </label>
          <input
            type="text"
            id="hallticket"
            value={hallticket}
            onChange={(e) => setHallticket(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
            disabled={isLoading}
            placeholder="e.g., value from your hallticket cookie"
          />
           <p className="mt-1 text-xs text-gray-500">
            This is typically the value part of your '''hallticket=YOUR_VALUE_HERE''' cookie.
          </p>
        </div>

        <div>
          <button
            type="submit"
            disabled={isLoading}
            className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:bg-indigo-300"
          >
            {isLoading ? 'Saving...' : 'Save Settings'}
          </button>
        </div>
      </form>
    </div>
  );
} 