import { describe, expect, it } from 'vitest';
import { chartsOf, seriesColour } from './charts';
import { CPU_ONLY, INTEGRATED, NVIDIA, TWO_GPUS, series } from './metrics.fixture';

const keys = (charts: ReturnType<typeof chartsOf>) => charts.map((chart) => chart.key);

describe('chartsOf', () => {
  it('MET-003: a CPU-only device has CPU and memory charts and nothing else', () => {
    expect(keys(chartsOf(CPU_ONLY, []))).toEqual(['cpu', 'memory']);
  });

  it('D-4: lists CPU, memory, mounts, network, load, then GPUs, without uptime', () => {
    expect(keys(chartsOf(TWO_GPUS, [NVIDIA, INTEGRATED]))).toEqual([
      'cpu',
      'memory',
      'disk:/',
      'disk:/data',
      'network',
      'load',
      'gpu:0:usage',
      'gpu:0:vram',
      'gpu:1:usage',
    ]);
  });

  it('D-4: one chart per mount, titled with the mount, and percent axis with the 85 % threshold (MET-004)', () => {
    const disks = chartsOf(TWO_GPUS, []).filter((chart) => chart.key.startsWith('disk:'));
    expect(disks.map((chart) => chart.title)).toEqual(['Disk /', 'Disk /data']);
    expect(disks[0]?.axis).toEqual({ kind: 'percent', min: 0, max: 100, threshold: 85 });
  });

  it('D-4: network has in and out, load has 1, 5 and 15 as separate series', () => {
    const charts = chartsOf(TWO_GPUS, []);
    const network = charts.find((chart) => chart.key === 'network');
    const load = charts.find((chart) => chart.key === 'load');
    expect(network?.series.map((line) => line.name)).toEqual(['In', 'Out']);
    expect(network?.axis).toEqual({ kind: 'open', unit: 'bytesPerSecond' });
    expect(network?.format(1024)).toBe('1.0 KiB/s');
    expect(load?.series.map((line) => line.name)).toEqual(['1 min', '5 min', '15 min']);
    expect(load?.series.map((line) => line.colour)).toEqual([1, 2, 3]);
  });

  it('MET-007: no VRAM chart for a GPU without its own memory, even when a series arrives', () => {
    const charts = chartsOf(TWO_GPUS, [NVIDIA, INTEGRATED]);
    expect(keys(charts)).not.toContain('gpu:1:vram');
    expect(keys(charts)).toContain('gpu:0:vram');
  });

  it('MET-009: GPU charts are titled with index, name and what they show', () => {
    const titles = chartsOf(TWO_GPUS, [NVIDIA, INTEGRATED])
      .filter((chart) => chart.key.startsWith('gpu:'))
      .map((chart) => chart.title);
    expect(titles).toEqual([
      'GPU 0 NVIDIA GeForce RTX 3060 usage',
      'GPU 0 NVIDIA GeForce RTX 3060 VRAM',
      'GPU 1 Intel UHD Graphics usage',
    ]);
  });

  it('lists no chart for a series that does not exist, and one for a series with no points', () => {
    expect(chartsOf([], [NVIDIA])).toEqual([]);
    expect(keys(chartsOf([series('CPU_PERCENT', 'cpu')], []))).toEqual(['cpu']);
  });

  it('cycles colours within the six series colours', () => {
    expect([0, 1, 5, 6, 7, 12].map(seriesColour)).toEqual([1, 2, 6, 1, 2, 1]);
  });
});
