import { describe, it, expect } from 'bun:test';
import { isValidUrl } from './actions';

describe('isValidUrl', () => {
	it('should return true for valid HTTP/HTTPS URLs', () => {
		expect(isValidUrl('http://example.com')).toBe(true);
		expect(isValidUrl('https://example.com')).toBe(true);
		expect(isValidUrl('https://www.example.com')).toBe(true);
		expect(isValidUrl('https://sub.domain.example.co.uk')).toBe(true);
		expect(isValidUrl('https://example.com/path/to/page')).toBe(true);
		expect(isValidUrl('https://example.com?query=string&another=1')).toBe(true);
		expect(isValidUrl('https://example.com#hash')).toBe(true);
		expect(isValidUrl('https://example.com:8080')).toBe(true);
	});

	it('should return true for domain-like patterns without protocol', () => {
		expect(isValidUrl('example.com')).toBe(true);
		expect(isValidUrl('www.example.com')).toBe(true);
		expect(isValidUrl('sub.domain.example.co.uk')).toBe(true);
		expect(isValidUrl('example.com/path')).toBe(true);
	});

	it('should handle edge cases', () => {
		expect(isValidUrl('localhost:3000')).toBe(true);
		expect(isValidUrl('127.0.0.1')).toBe(true);
		expect(isValidUrl('  https://example.com  ')).toBe(true);
	});

	it('should return false for invalid URLs and strings', () => {
		expect(isValidUrl('')).toBe(false);
		expect(isValidUrl('just a normal string')).toBe(false);
		expect(isValidUrl('javascript:alert(1)')).toBe(false);
		expect(isValidUrl('mailto:test@example.com')).toBe(false);
		expect(isValidUrl('data:text/html,<h1>Hello</h1>')).toBe(false);
	});
});
