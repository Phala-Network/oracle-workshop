const fs = require('fs');
const crypto = require('crypto');
const { checkUntil, checkUntilEq, hex } = require('./utils');

const CONTRACT_NAMES = [
    ['fat_badges', 'FatBadges'],
    ['easy_oracle', 'EasyOracle'],
    ['advanced_judger', 'AdvancedJudger'],
]

function loadContract(name) {
    const wasmPath = `../../target/ink/${name}/${name}.wasm`;
    const metadataPath = `../../target/ink/${name}/metadata.json`;
    const wasm = hex(fs.readFileSync(wasmPath, 'hex'));
    const metadata = JSON.parse(fs.readFileSync(metadataPath));
    const constructor = metadata.V3.spec.constructors.find(c => c.label == 'new').selector;
    return {wasm, metadata, constructor};
}

function loadArtifacts() {
    return Object.assign(
        {}, ...CONTRACT_NAMES.map(
            ([filename, name]) => ({[name]: loadContract(filename)})
        )
    );
}

async function deployContracts(api, txqueue, pair, artifacts, clusterId) {
    console.log('Contracts: uploading');
    // upload contracts
    const contractNames = Object.keys(artifacts);
    const { events: deployEvents } = await txqueue.submit(
        api.tx.utility.batchAll(
            Object.entries(artifacts).flatMap(([_k, v]) => [
                api.tx.phalaFatContracts.clusterUploadResource(clusterId, 'InkCode', v.wasm),
                api.tx.phalaFatContracts.instantiateContract(
                    { WasmCode: v.metadata.source.hash },
                    v.constructor,
                    hex(crypto.randomBytes(4).toString('hex')), // salt
                    clusterId,
                )
            ])
        ),
        pair
    );
    const contractIds = deployEvents
        .filter(ev => ev.event.section == 'phalaFatContracts' && ev.event.method == 'Instantiating')
        .map(ev => ev.event.data[0].toString());
    const numContracts = contractNames.length;
    console.assert(contractIds.length == numContracts, 'Incorrect length:', `${contractIds.length} vs ${numContracts}`);
    for (const [i, id] of contractIds.entries()) {
        artifacts[contractNames[i]].address = id;
    }
    await checkUntilEq(
        async () => (await api.query.phalaFatContracts.clusterContracts(clusterId))
            .filter(c => contractIds.includes(c.toString()) )
            .length,
        numContracts,
        4 * 6000
    );
    console.log('Contracts: uploaded');
    for (const [name, contract] of Object.entries(artifacts)) {
        await checkUntil(
            async () => (await api.query.phalaRegistry.contractKeys(contract.address)).isSome,
            4 * 6000
        );
        console.log('Contracts:', contract.address, name, 'key ready');
    }
    console.log('Contracts: deployed');
}

module.exports = {
    loadArtifacts,
    deployContracts,
}
