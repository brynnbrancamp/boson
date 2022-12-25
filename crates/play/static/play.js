import init from "/engine/engine.js";
import * as engine from "/engine/engine.js";

async function run() {
	var runtime = await engine.start();
	console.log(runtime);

	async function frame() {
		await runtime.next();
		requestAnimationFrame(frame);
	}

	requestAnimationFrame(frame);
}

init().then(run)
