require('dotenv').config();
const fs = require('fs');

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {ContractPromise} = require('@polkadot/api-contract');
const Phala = require('@phala/sdk');

const { TxQueue, blockBarrier, hex } = require('./utils');
const { loadArtifacts, deployContracts } = require('./common');

function loadCode(path) {
    const content = fs.readFileSync(path, {encoding: 'utf-8'});
    return content.split('\n').map(x => x.trim()).filter(x => !!x);
}

async function main() {
    const clusterId = process.env.CLUSTER_ID || '0x0000000000000000000000000000000000000000000000000000000000000000';
    const privkey = process.env.PRIVKEY || '//Alice';
    const chainUrl = process.env.CHAIN || 'wss://poc5.phala.network/ws';
    const pruntimeUrl = process.env.PRUNTIME || 'https://poc5.phala.network/tee-api-1';
    const codeEasyCsv = process.env.CODE_EASY_CSV || './tmp/code-easy.csv';
    const codeAdvCsv = process.env.CODE_EASY_CSV || './tmp/code-adv.csv';

    const artifacts = loadArtifacts();
    const codeEasy = loadCode(codeEasyCsv);
    const codeAdv = loadCode(codeAdvCsv);

    // connect to the chain
    const wsProvider = new WsProvider(chainUrl);
    const api = await ApiPromise.create({
        provider: wsProvider,
        types: {
            ...Phala.types,
            'GistQuote': {
                username: 'String',
                accountId: 'AccountId',
            },
        }
    });
    const txqueue = new TxQueue(api);

    // prepare accounts
    const keyring = new Keyring({type: 'sr25519'})
    const pair = keyring.addFromUri(privkey);
    const cert = await Phala.signCertificate({api, pair});

    // connect to pruntime
    const prpc = Phala.createPruntimeApi(pruntimeUrl);
    const connectedWorker = hex((await prpc.getInfo({})).publicKey);
    console.log('Connected worker:', connectedWorker);

    // contracts
    await deployContracts(api, txqueue, pair, artifacts, clusterId);
    
    // create Fat Contract objects
    const contracts = {};
    for (const [name, contract] of Object.entries(artifacts)) {
        const contractId = contract.address;
        const newApi = await api.clone().isReady;
        contracts[name] = new ContractPromise(
            await Phala.create({api: newApi, baseURL: pruntimeUrl, contractId}),
            contract.metadata,
            contractId
        );
    }
    console.log('Fat Contract: connected');
    const { FatBadges, EasyOracle, AdvancedJudger } = contracts;

    
    // set up the contracts
    const easyBadgeId = 0;
    const advBadgeId = 1;
    await txqueue.submit(
        api.tx.utility.batchAll([
            // set up the badges; assume the ids are 0 and 1.
            FatBadges.tx.newBadge({}, 'fat-easy-challenge'),
            FatBadges.tx.newBadge({}, 'fat-adv-challenge'),
            // fill with code
            FatBadges.tx.addCode({}, easyBadgeId, codeEasy),
            FatBadges.tx.addCode({}, advBadgeId, codeAdv),
            // set the issuers
            FatBadges.tx.addIssuer({}, easyBadgeId, artifacts.EasyOracle.address),
            FatBadges.tx.addIssuer({}, advBadgeId, artifacts.AdvancedJudger.address),
            // config the issuers
            EasyOracle.tx.configIssuer({}, artifacts.FatBadges.address, easyBadgeId),
            AdvancedJudger.tx.configIssuer({}, artifacts.FatBadges.address, advBadgeId),
        ]),
        pair,
        true,
    );

    // wait for the worker to sync to the bockchain
    await blockBarrier(api, prpc);

    // basic checks
    console.log('Fat Contract: basic checks');
    console.assert(
        (await FatBadges.query.getTotalBadges(cert, {})).output.toNumber() == 2,
        'Should have two badges created'
    );

    const easyInfo = await FatBadges.query.getBadgeInfo(cert, {}, easyBadgeId);
    console.log('Easy badge:', easyInfo.output.toHuman());

    const advInfo = await FatBadges.query.getBadgeInfo(cert, {}, advBadgeId);
    console.log('Adv badge:', advInfo.output.toHuman());

    console.log('Deployment finished');
}

main().then(process.exit).catch(err => console.error('Crashed', err)).finally(() => process.exit(-1));