import { describe, expect, it } from 'vitest';
import { chartsOf } from './overview-charts';
import { APPLE, CPU_ONLY, INTEGRATED, NVIDIA, point, TWO_GPUS, series } from './metrics.fixture';

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
      'network:eth0',
      'load',
      'gpu:0:usage',
      'gpu:0:vram',
      'gpu:1:usage',
    ]);
  });

  it('D-4: one chart per mount, titled with the mount, and percent axis with the 85 % threshold (MET-004)', () => {
    const disks = chartsOf(TWO_GPUS, []).filter((chart) => chart.key.startsWith('disk:'));
    expect(disks.map((chart) => chart.title)).toEqual(['Disk /', 'Disk /data']);
    expect(disks[0]?.axis).toBe('percent');
    expect(disks[0]?.threshold).toBe(85);
  });

  it('D-4: network has in and out, load has 1, 5 and 15 as separate series', () => {
    const charts = chartsOf(TWO_GPUS, []);
    const network = charts.find((chart) => chart.key === 'network:eth0');
    const load = charts.find((chart) => chart.key === 'load');
    expect(network?.series.map((line) => line.label)).toEqual(['In', 'Out']);
    expect(network?.axis).toBe('open');
    expect(network?.threshold).toBeUndefined();
    expect(network?.format(1024)).toBe('1.0 KiB/s');
    expect(load?.series.map((line) => line.label)).toEqual(['1 min', '5 min', '15 min']);
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

});

describe('chartsOf series', () => {
  it('D-4: draws one In and Out chart per interface, ordered by name, each from its own interface', () => {
    const at = '2026-11-05T09:04:00Z';
    const charts = chartsOf(
      [
        series('NETWORK_OUT_BPS', 'wlan0', [point(at, 2)]),
        series('NETWORK_IN_BPS', 'wlan0', [point(at, 1)]),
        series('NETWORK_IN_BPS', 'eth0', [point(at, 10)]),
        series('NETWORK_OUT_BPS', 'eth0', [point(at, 20)]),
      ],
      [],
    );
    expect(charts.map((chart) => chart.title)).toEqual(['Network eth0', 'Network wlan0']);
    expect(charts[1]?.series.map((line) => [line.label, line.points[0]?.value])).toEqual([
      ['In', 1],
      ['Out', 2],
    ]);
  });

  it('MET-003: points are in milliseconds and sorted oldest first', () => {
    const late = point('2026-11-05T09:04:00Z', 2);
    const early = point('2026-11-05T09:00:00Z', 1);
    const [chart] = chartsOf([series('CPU_PERCENT', 'cpu', [late, early])], []);
    expect(chart?.series[0]?.points.map((each) => each.at)).toEqual([Date.parse(early.at), Date.parse(late.at)]);
  });

  it('charts a CPU series with an empty label, the form WP-28 D-5 serves', () => {
    const [chart] = chartsOf([series('CPU_PERCENT', '', [point('2026-11-05T09:00:00Z', 5)])], []);
    expect(chart?.key).toBe('cpu');
    expect(chart?.series[0]?.label).toBe('CPU');
  });

  it('gives the lines of a chart colours 1, 2, 3 in order', () => {
    const charts = chartsOf(TWO_GPUS, []);
    const colours = (key: string) => charts.find((chart) => chart.key === key)?.series.map((line) => line.colour);
    expect(colours('network:eth0')).toEqual([1, 2]);
    expect(colours('load')).toEqual([1, 2, 3]);
  });
});

describe('chartsOf shared GPU memory', () => {
  it('D-30: a GPU with shared memory gets a bytes chart for it, and no VRAM chart', () => {
    const at = '2026-11-05T09:04:00Z';
    const charts = chartsOf([series('GPU_PERCENT', 'GPU 0', [point(at, 10)], 0), series('GPU_MEMORY_BYTES', 'GPU 0', [point(at, 2 ** 30)], 0)], [APPLE]);
    expect(keys(charts)).toEqual(['gpu:0:usage', 'gpu:0:memory']);
    expect(charts[1]?.title).toBe('GPU 0 Apple M2 Pro memory in use (shared)');
    expect(charts[1]?.axis).toBe('open');
    expect(charts[1]?.format(2 ** 30)).toBe('1.0 GiB');
  });

  it('D-30, MET-007: a GPU with its own memory gets no shared memory chart, even when that series arrives', () => {
    const at = '2026-11-05T09:04:00Z';
    const charts = chartsOf([series('GPU_PERCENT', 'GPU 0', [point(at, 10)], 0), series('GPU_MEMORY_BYTES', 'GPU 0', [point(at, 2 ** 30)], 0)], [NVIDIA]);
    expect(keys(charts)).toEqual(['gpu:0:usage']);
  });
});
