export interface Transaction {
  id: string; // i64 can be large, string is safer for IDs from backend
  time: string; // ISO 8601 date string
  amount: number;
  merchant: string;
}

export interface FilterOptions {
  time?: [string, string]; // [startDate, endDate] ISO 8601 date strings
  merchant?: string;
  amount?: [number, number]; // [minAmount, maxAmount]
}

export interface FetchTransactionsRequest {
  start_date: string; // ISO 8601 date string
}

export interface AccountUpdateRequest {
  account: string;
}

export interface HallticketUpdateRequest {
  hallticket: string;
}

export interface AccountCookieResponse {
  account: string;
  cookie: string;
}

// Types for Analysis Data (to be defined based on src/page/analysis.rs)
// Placeholder for now, will be filled based on Rust structs TimePeriodData, TimeSeriesData, etc.

export interface TimePeriodSummary {
  period: string; // e.g., "This Month", "Last Week"
  totalSpent: number;
  transactionCount: number;
  averageSpent: number;
}

export interface TimeSeriesDataPoint {
  date: string; // e.g., "YYYY-MM-DD"
  amount: number;
}

export interface MerchantSpending {
  merchant: string;
  totalSpent: number;
  transactionCount: number;
}

export interface MerchantCategorySpending {
  category: string; // Assuming categories can be derived or are predefined
  totalSpent: number;
  transactionCount: number;
}

// General type for chart data, can be adapted
export interface ChartData {
  name: string; // Label for X-axis or segment
  value: number; // Value for Y-axis or segment
  // Optional fill color for charts if needed per segment
  fill?: string;
}

// Specific data structure for Time Period Analysis results
export interface ProcessedTimePeriodData {
  breakfast: number;
  lunch: number;
  dinner: number;
  unknown: number;
  chartData: ChartData[];
}

export interface ProcessedTimeSeriesData {
  chartData: ChartData[]; // Expects { name: "YYYY-MM", value: number }[]
}

export interface ProcessedMerchantData {
  chartData: ChartData[];
  rawData: { merchant: string; totalAmount: number }[];
}

// TODO: Define specific processed data types for MerchantCategory as it is implemented 