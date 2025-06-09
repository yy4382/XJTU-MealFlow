import type { Transaction, FilterOptions, FetchTransactionsRequest, AccountUpdateRequest, HallticketUpdateRequest, AccountCookieResponse } from "./types";

const API_BASE_URL = "/api"; // Assuming the Vite proxy is set up or a relative path works

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(errorData.message || `HTTP error! status: ${response.status}`);
  }
  if (response.status === 204 || response.headers.get("content-length") === "0") {
    // Handle cases where backend returns 200/204 with no content
    // or an actual empty body for success where no data is expected.
    return undefined as T;
  }
  return response.json();
}

// Transaction APIs
export const fetchAllTransactions = async (): Promise<Transaction[]> => {
  const response = await fetch(`${API_BASE_URL}/transactions`);
  return handleResponse<Transaction[]>(response);
};

export const fetchFilteredTransactions = async (filterOpts: FilterOptions): Promise<Transaction[]> => {
  const response = await fetch(`${API_BASE_URL}/transactions/query`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(filterOpts),
  });
  return handleResponse<Transaction[]>(response);
};

export const fetchTransactionCount = async (): Promise<number> => {
  const response = await fetch(`${API_BASE_URL}/transactions/count`);
  const data = await handleResponse<any>(response); // Use any for initial parsing, then check type

  if (typeof data === 'number') {
    return data;
  }
  if (typeof data === 'string') { // Rust u64 might be serialized as string
    const parsed = parseInt(data, 10);
    if (!isNaN(parsed)) {
      return parsed;
    }
  }
  if (data && typeof data.count === 'number') {
    return data.count;
  }
  if (data && typeof data.count === 'string') { // Rust u64 in an object might be serialized as string
    const parsed = parseInt(data.count, 10);
    if (!isNaN(parsed)) {
      return parsed;
    }
  }
  console.warn('Unexpected transaction count format:', data);
  return 0; // Default or throw error
};

export const triggerFetchTransactions = async (request: FetchTransactionsRequest): Promise<void> => {
  const response = await fetch(`${API_BASE_URL}/transactions/fetch`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(request),
  });
  await handleResponse<void>(response); // Expecting no content on success
};

// Config APIs
export const updateAccount = async (request: AccountUpdateRequest): Promise<void> => {
  const response = await fetch(`${API_BASE_URL}/config/account`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(request),
  });
  await handleResponse<void>(response);
};

export const updateHallticket = async (request: HallticketUpdateRequest): Promise<void> => {
  const response = await fetch(`${API_BASE_URL}/config/hallticket`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(request),
  });
  await handleResponse<void>(response);
};

export const getAccountCookie = async (): Promise<AccountCookieResponse> => {
  const response = await fetch(`${API_BASE_URL}/config/account-cookie`);
  // This endpoint might return 404 which handleResponse will throw as error, this is fine.
  return handleResponse<AccountCookieResponse>(response);
};

// CSV Export API
export interface CsvExportParams {
  merchant?: string;
  min_amount?: number;
  max_amount?: number;
  time_start?: string; // YYYY-MM-DD format
  time_end?: string; // YYYY-MM-DD format
  format?: 'csv' | 'json';
}

export const exportCsv = async (params: CsvExportParams = {}): Promise<Blob> => {
  const searchParams = new URLSearchParams();
  
  if (params.merchant) searchParams.append('merchant', params.merchant);
  if (params.min_amount !== undefined) searchParams.append('min_amount', params.min_amount.toString());
  if (params.max_amount !== undefined) searchParams.append('max_amount', params.max_amount.toString());
  if (params.time_start) searchParams.append('time_start', params.time_start);
  if (params.time_end) searchParams.append('time_end', params.time_end);
  if (params.format) searchParams.append('format', params.format);

  const response = await fetch(`${API_BASE_URL}/export/csv?${searchParams.toString()}`);
  
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(errorData.message || `HTTP error! status: ${response.status}`);
  }
  
  return response.blob();
}; 