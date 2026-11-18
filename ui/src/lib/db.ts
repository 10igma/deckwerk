import type { Db } from '../core/types';

// Umrechnung zwischen Übertragungsformat (null = −∞) und Rechenwert.
export const fromWire = (db: Db): number => (db === null ? -Infinity : db);
export const toWire = (db: number): Db => (Number.isFinite(db) ? db : null);

export const dbToLin = (db: number): number => (db === -Infinity ? 0 : Math.pow(10, db / 20));
export const linToDb = (lin: number): number => (lin <= 0 ? -Infinity : 20 * Math.log10(lin));
